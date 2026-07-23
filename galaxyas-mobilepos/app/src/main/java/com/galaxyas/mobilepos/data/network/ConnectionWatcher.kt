package com.galaxyas.mobilepos.data.network

import com.galaxyas.mobilepos.data.ServerRegistry
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * Pemantau keterjangkauan Server Pusat untuk banner "tidak terhubung" — HP
 * kasir jauh lebih sering lepas wifi daripada PC. Ping /health tiap 30 detik;
 * layar shell juga memanggil checkNow() saat ON_RESUME dan tombol "Coba lagi".
 */
class ConnectionWatcher(
    private val rpc: RpcClient,
    private val registry: ServerRegistry,
    private val scope: CoroutineScope,
) {
    private val _reachable = MutableStateFlow(true)
    val reachable: StateFlow<Boolean> = _reachable.asStateFlow()

    private val _checking = MutableStateFlow(false)
    val checking: StateFlow<Boolean> = _checking.asStateFlow()

    fun start() {
        scope.launch {
            while (true) {
                checkNow()
                delay(30_000)
            }
        }
    }

    suspend fun checkNow(): Boolean {
        val server = registry.activeServer.value ?: return true // belum pairing — bukan urusan banner
        _checking.value = true
        return try {
            rpc.healthCheck(server.host, server.port, server.token)
            _reachable.value = true
            true
        } catch (_: Exception) {
            _reachable.value = false
            false
        } finally {
            _checking.value = false
        }
    }

    fun checkAsync() {
        scope.launch { checkNow() }
    }
}
