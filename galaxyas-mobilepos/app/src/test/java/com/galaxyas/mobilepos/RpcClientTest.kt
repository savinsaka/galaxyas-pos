package com.galaxyas.mobilepos

import com.galaxyas.mobilepos.data.network.RemoteConfig
import com.galaxyas.mobilepos.data.network.RpcAuthException
import com.galaxyas.mobilepos.data.network.RpcClient
import com.galaxyas.mobilepos.data.network.RpcException
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Before
import org.junit.Test

class RpcClientTest {
    private lateinit var server: MockWebServer
    private lateinit var rpc: RpcClient
    private lateinit var remote: RemoteConfig

    @Before
    fun setUp() {
        server = MockWebServer()
        server.start()
        remote = RemoteConfig(server.hostName, server.port, "ABC123")
        rpc = RpcClient { remote }
    }

    @After
    fun tearDown() {
        server.shutdown()
    }

    @Test
    fun `call mengirim token header, path rpc, dan body args`() = runTest {
        server.enqueue(MockResponse().setBody("""[{"id":"b1","name":"Merek A","updated_at":""}]"""))

        val result = rpc.call("list_brands", buildJsonObject { put("search", "abc") })

        val req = server.takeRequest()
        assertEquals("/rpc/list_brands", req.path)
        assertEquals("POST", req.method)
        assertEquals("ABC123", req.getHeader(RpcClient.TOKEN_HEADER))
        assertEquals("""{"search":"abc"}""", req.body.readUtf8())
        assertEquals("Merek A", result.jsonArray[0].jsonObject["name"]!!.jsonPrimitive.content)
    }

    @Test
    fun `body respons kosong dianggap JsonNull`() = runTest {
        server.enqueue(MockResponse().setBody(""))
        val result = rpc.call("delete_brand", buildJsonObject { put("id", "x") })
        assertEquals(JsonNull, result)
    }

    @Test
    fun `error 400 menampilkan pesan dari field error`() = runTest {
        server.enqueue(MockResponse().setResponseCode(400).setBody("""{"error":"stok tidak cukup"}"""))
        try {
            rpc.call("checkout", JsonNull)
            fail("harus melempar RpcException")
        } catch (e: RpcException) {
            assertEquals("stok tidak cukup", e.message)
        }
    }

    @Test
    fun `error 401 menjadi RpcAuthException`() = runTest {
        server.enqueue(MockResponse().setResponseCode(401).setBody("""{"error":"unauthorized"}"""))
        try {
            rpc.call("list_users", JsonNull)
            fail("harus melempar RpcAuthException")
        } catch (e: RpcAuthException) {
            assertEquals("unauthorized", e.message)
        }
    }

    @Test
    fun `error koneksi memakai pesan Indonesia verbatim dari lan rs`() = runTest {
        server.shutdown() // koneksi pasti ditolak
        try {
            rpc.call("list_brands", JsonNull)
            fail("harus melempar RpcException")
        } catch (e: RpcException) {
            assertEquals(RpcClient.CONNECT_ERR_MSG, e.message)
        }
    }

    @Test
    fun `belum pairing melempar pesan belum terhubung`() = runTest {
        val unpaired = RpcClient { null }
        try {
            unpaired.call("list_brands", JsonNull)
            fail("harus melempar RpcException")
        } catch (e: RpcException) {
            assertEquals(RpcClient.NOT_PAIRED_MSG, e.message)
        }
    }

    @Test
    fun `healthCheck 401 melempar kode pairing salah`() = runTest {
        server.enqueue(MockResponse().setResponseCode(401).setBody("""{"error":"kode pairing salah"}"""))
        try {
            rpc.healthCheck(server.hostName, server.port, "SALAH1")
            fail("harus melempar RpcAuthException")
        } catch (e: RpcAuthException) {
            assertEquals("kode pairing salah", e.message)
        }
        assertEquals("/health", server.takeRequest().path)
    }

    @Test
    fun `healthCheck sukses tidak melempar`() = runTest {
        server.enqueue(MockResponse().setBody("""{"ok":true,"authorized":true}"""))
        rpc.healthCheck(server.hostName, server.port, "ABC123")
        assertTrue(true)
    }
}
