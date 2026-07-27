package com.galaxyas.mobilepos

import com.galaxyas.mobilepos.data.network.ConnMode
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
        remote = RemoteConfig("http://${server.hostName}:${server.port}", "ABC123", ConnMode.LOCAL)
        rpc = RpcClient { remote }
    }

    @After
    fun tearDown() {
        server.shutdown()
    }

    /** Jalur ONLINE: base url membawa prefiks `/s/<store_id>` milik relay. */
    private fun onlineRemote(token: String = "tokenpanjang") =
        RemoteConfig("http://${server.hostName}:${server.port}/s/toko123", token, ConnMode.ONLINE)

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
    fun `mode ONLINE memakai prefiks store id relay`() = runTest {
        server.enqueue(MockResponse().setBody("[]"))
        rpc.callTo(onlineRemote(), "list_brands", JsonNull)
        val req = server.takeRequest()
        assertEquals("/s/toko123/rpc/list_brands", req.path)
        assertEquals("tokenpanjang", req.getHeader(RpcClient.TOKEN_HEADER))
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

    /** 503 dari relay = PC kasir mati. Pesannya sudah siap tampil, jangan ditimpa. */
    @Test
    fun `503 dari relay diteruskan apa adanya ke user`() = runTest {
        server.enqueue(
            MockResponse().setResponseCode(503)
                .setBody("""{"error":"PC kasir sedang mati atau tidak terhubung internet."}"""),
        )
        try {
            rpc.callTo(onlineRemote(), "checkout", JsonNull)
            fail("harus melempar RpcException")
        } catch (e: RpcException) {
            assertEquals("PC kasir sedang mati atau tidak terhubung internet.", e.message)
            assertTrue("503 bukan masalah kredensial", e !is RpcAuthException)
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

    /** Di mode ONLINE, menyuruh user "periksa wifi" menyesatkan. */
    @Test
    fun `error koneksi mode ONLINE menyebut internet HP, bukan wifi toko`() = runTest {
        val online = onlineRemote()
        server.shutdown()
        try {
            rpc.callTo(online, "list_brands", JsonNull)
            fail("harus melempar RpcException")
        } catch (e: RpcException) {
            assertEquals(RpcClient.CONNECT_ERR_MSG_ONLINE, e.message)
            assertTrue(e.message!!.contains("internet"))
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
            rpc.healthCheck(remote)
            fail("harus melempar RpcAuthException")
        } catch (e: RpcAuthException) {
            assertEquals("kode pairing salah", e.message)
        }
        assertEquals("/health", server.takeRequest().path)
    }

    @Test
    fun `healthCheck sukses tidak melempar`() = runTest {
        server.enqueue(MockResponse().setBody("""{"ok":true,"authorized":true}"""))
        rpc.healthCheck(remote)
        assertTrue(true)
    }

    @Test
    fun `healthCheck mode ONLINE menembak health milik relay`() = runTest {
        server.enqueue(MockResponse().setBody("""{"ok":true}"""))
        rpc.healthCheck(onlineRemote())
        assertEquals("/s/toko123/health", server.takeRequest().path)
    }

    @Test
    fun `pair menukar kode pendek dengan token perangkat`() = runTest {
        server.enqueue(
            MockResponse().setBody(
                """{"device_id":"d1","device_token":"${"a".repeat(64)}","store_name":"Toko Uji"}""",
            ),
        )
        val result = rpc.pair(remote, "A1B2C3", "Redmi Note 12")

        val req = server.takeRequest()
        assertEquals("/pair", req.path)
        assertEquals("POST", req.method)
        assertEquals("""{"code":"A1B2C3","device_name":"Redmi Note 12"}""", req.body.readUtf8())
        assertEquals(64, result.device_token.length)
        assertEquals("Toko Uji", result.store_name)
    }

    @Test
    fun `pair dengan kode salah melempar RpcAuthException`() = runTest {
        server.enqueue(
            MockResponse().setResponseCode(401).setBody("""{"error":"kode pairing salah"}"""),
        )
        try {
            rpc.pair(remote, "SALAH1", "Redmi")
            fail("harus melempar RpcAuthException")
        } catch (e: RpcAuthException) {
            assertEquals("kode pairing salah", e.message)
        }
    }
}
