package com.galaxyas.mobilepos

import android.app.Application
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

class AppContainer(app: Application) {
    val appScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val serverRegistry = ServerRegistry(app, appScope)
    val settings = SettingsRepository(app, appScope)
    val session = Session()
    val rpc = RpcClient { serverRegistry.activeRemote() }
    val api = ApiClient(rpc)
    val connectionWatcher = ConnectionWatcher(rpc, serverRegistry, appScope)
}
