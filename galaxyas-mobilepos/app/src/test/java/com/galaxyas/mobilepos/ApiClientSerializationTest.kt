package com.galaxyas.mobilepos

import com.galaxyas.mobilepos.data.model.ProductWithStock
import com.galaxyas.mobilepos.data.model.SaleInput
import com.galaxyas.mobilepos.data.model.SaleItemInput
import com.galaxyas.mobilepos.data.model.TransactionDetail
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.data.network.RemoteConfig
import com.galaxyas.mobilepos.data.network.RpcClient
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test

/**
 * Uji bentuk WIRE dari argumen ApiClient — nama arg harus persis nama parameter
 * command Rust (snake_case) dan pembungkusan input ({"sale":…}, {"input":…})
 * harus sama seperti desktop api.ts + lan.rs::dispatch.
 */
class ApiClientSerializationTest {
    private lateinit var server: MockWebServer
    private lateinit var api: ApiClient

    private val json = Json { ignoreUnknownKeys = true }

    @Before
    fun setUp() {
        server = MockWebServer()
        server.start()
        val remote = RemoteConfig(server.hostName, server.port, "T")
        api = ApiClient(RpcClient { remote })
    }

    @After
    fun tearDown() {
        server.shutdown()
    }

    @Test
    fun `listProductsPage memakai arg wire snake_case`() = runTest {
        server.enqueue(MockResponse().setBody("""{"items":[],"total":0}"""))
        api.listProductsPage(search = "kopi", includeInactive = true, brand = "A", limit = 50, offset = 100)

        val body = json.parseToJsonElement(server.takeRequest().body.readUtf8()).jsonObject
        assertEquals("kopi", body["search"]!!.jsonPrimitive.content)
        assertEquals("true", body["include_inactive"]!!.jsonPrimitive.content)
        assertEquals("A", body["brand"]!!.jsonPrimitive.content)
        assertEquals("name", body["sort_by"]!!.jsonPrimitive.content)
        assertEquals("asc", body["sort_dir"]!!.jsonPrimitive.content)
        assertEquals("50", body["limit"]!!.jsonPrimitive.content)
        assertEquals("100", body["offset"]!!.jsonPrimitive.content)
        // camelCase TIDAK boleh bocor ke wire
        assertNull(body["includeInactive"])
        assertNull(body["sortBy"])
    }

    @Test
    fun `checkout membungkus SaleInput dalam field sale`() = runTest {
        server.enqueue(
            MockResponse().setBody(
                """
                {"id":"t1","invoice_no":"INV-1","cashier_id":"kasir","subtotal":10000.0,
                 "discount":0.0,"total":10000.0,"paid":10000.0,"change":0.0,
                 "payment_method":"Tunai","created_at":"2026-07-23T10:00:00",
                 "items":[{"product_id":"p1","name":"Kopi","price":10000.0,"qty":1.0,
                           "discount":0.0,"line_total":10000.0}]}
                """.trimIndent(),
            ),
        )

        val detail: TransactionDetail = api.checkout(
            SaleInput(
                cashier_id = "kasir",
                payment_method = "Kombinasi",
                paid = 10000.0,
                items = listOf(SaleItemInput("p1", "Kopi", 10000.0, 1.0, 0.0)),
                shift_id = "s1",
                paid_cash = 6000.0,
                paid_qris = 4000.0,
            ),
        )

        val req = server.takeRequest()
        assertEquals("/rpc/checkout", req.path)
        val body = json.parseToJsonElement(req.body.readUtf8()).jsonObject
        val sale = body["sale"]!!.jsonObject
        assertEquals("kasir", sale["cashier_id"]!!.jsonPrimitive.content)
        assertEquals("Kombinasi", sale["payment_method"]!!.jsonPrimitive.content)
        assertEquals("6000.0", sale["paid_cash"]!!.jsonPrimitive.content)
        assertEquals("4000.0", sale["paid_qris"]!!.jsonPrimitive.content)
        assertEquals("s1", sale["shift_id"]!!.jsonPrimitive.content)
        val firstItem = sale["items"]!!.jsonArray[0].jsonObject
        assertEquals("p1", firstItem["product_id"]!!.jsonPrimitive.content)
        assertEquals("Kopi", firstItem["name"]!!.jsonPrimitive.content)
        assertEquals("INV-1", detail.invoice_no)
    }

    @Test
    fun `decode ProductWithStock toleran field tak dikenal`() {
        val p = json.decodeFromString<ProductWithStock>(
            """
            {"id":"p1","name":"Kopi","barcode":null,"category":null,"brand":"A","unit":"pcs",
             "sell_price":15000.0,"cost_price":10000.0,"default_discount":0.0,
             "is_active":true,"is_deleted":false,"updated_at":"2026-01-01",
             "stock_qty":7.5,"field_baru_dari_server":123}
            """.trimIndent(),
        )
        assertEquals(7.5, p.stock_qty, 0.0)
        assertEquals("A", p.brand)
    }
}
