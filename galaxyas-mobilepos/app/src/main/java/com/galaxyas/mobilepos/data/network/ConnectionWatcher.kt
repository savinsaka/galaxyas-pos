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
 *
 * Mengikuti jalur yang sedang dipilih (LOCAL/ONLINE). Di jalur ONLINE, /health
 * dijawab relay sendiri, jadi denyut ini tidak membebani PC kasir dan hemat
 * kuota.
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

    /** Pesan kegagalan terakhir — dibedakan LOCAL vs ONLINE, jadi ditampilkan. */
    private val _lastError = MutableStateFlow<String?>(null)
    val lastError: StateFlow<String?> = _lastError.asStateFlow()

    fun start() {
        scope.launch {
            while (true) {
                checkNow()
                delay(30_000)
            }
        }
    }

    suspend fun checkNow(): Boolean {
        // Belum pairing / jalur aktif belum lengkap — bukan urusan banner.
        val remote = registry.activeRemote() ?: return true
        _checking.value = true
        return try {
            rpc.healthCheck(remote)
            _reachable.value = true
            _lastError.value = null
            true
        } catch (e: Exception) {
            _reachable.value = false
            _lastError.value = e.message ?: RpcClient.connectErrorFor(remote.mode)
            false
        } finally {
            _checking.value = false
        }
    }

    fun checkAsync() {
        scope.launch { checkNow() }
    }
}
