package com.galaxyas.mobilepos

import android.app.Application
import com.galaxyas.mobilepos.data.PendingSalesStore
import com.galaxyas.mobilepos.data.ServerRegistry
import com.galaxyas.mobilepos.data.Session
import com.galaxyas.mobilepos.data.SettingsRepository
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.data.network.ConnectionWatcher
import com.galaxyas.mobilepos.data.network.RpcClient
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob

/** Application + wiring dependency manual (tanpa Hilt — app kecil). */
class PosApp : Application() {
    lateinit var container: AppContainer
        private set

    override fun onCreate() {
        super.onCreate()
        container = AppContainer(this)
        container.connectionWatcher.start()
    }
}

/**
 * Titipan objek antar-layar saat navigasi (Compose Navigation hanya membawa
 * argumen primitif). Dipakai daftar barang -> form edit supaya tidak refetch.
 */
class UiBuffer {
    var product: com.galaxyas.mobilepos.data.model.ProductWithStock? = null
    var opnameBrand: String? = null
}

class AppContainer(app: Application) {
    val appScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val buffer = UiBuffer()
    val serverRegistry = ServerRegistry(app, appScope)
    val settings = SettingsRepository(app, appScope)
    val pendingSales = PendingSalesStore(app, appScope)
    val session = Session()
    val appContext: android.content.Context = app.applicationContext
    val rpc = RpcClient { serverRegistry.activeRemote() }
    val api = ApiClient(rpc)
    val connectionWatcher = ConnectionWatcher(rpc, serverRegistry, appScope)
}
