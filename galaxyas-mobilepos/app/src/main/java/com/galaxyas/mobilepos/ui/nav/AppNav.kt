package com.galaxyas.mobilepos.ui.nav

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import com.galaxyas.mobilepos.AppContainer
import com.galaxyas.mobilepos.ui.common.ComingSoon
import com.galaxyas.mobilepos.ui.common.ConnectionBanner
import com.galaxyas.mobilepos.ui.login.LoginScreen
import com.galaxyas.mobilepos.ui.menu.MenuScreen
import com.galaxyas.mobilepos.ui.onboarding.PairingScreen
import com.galaxyas.mobilepos.ui.produk.ProductListScreen
import com.galaxyas.mobilepos.ui.produk.ProductListViewModel

private data class Tab(
    val route: String,
    val label: String,
    val icon: String,
    val perm: String?, // null = selalu tampil
)

private val TABS = listOf(
    Tab("kasir", "Kasir", "🧾", "penjualan"),
    Tab("produk", "Produk", "📦", "master"),
    Tab("laporan", "Laporan", "📊", "laporan"),
    Tab("menu", "Menu", "☰", null),
)

/** Gate boot: belum pairing → Pairing; belum login → Login; else shell tab. */
@Composable
fun AppRoot(container: AppContainer) {
    val activeServer by container.serverRegistry.activeServer.collectAsState()
    val user by container.session.user.collectAsState()
    var forcePairing by remember { mutableStateOf(false) }

    when {
        activeServer == null || forcePairing -> PairingScreen(
            registry = container.serverRegistry,
            rpc = container.rpc,
            onPaired = {
                forcePairing = false
                container.session.logout()
                container.connectionWatcher.checkAsync()
            },
        )
        user == null -> LoginScreen(
            api = container.api,
            session = container.session,
            registry = container.serverRegistry,
            onChangeServer = { forcePairing = true },
        )
        else -> MainShell(container, onChangeServer = {
            container.session.logout()
            forcePairing = true
        })
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun MainShell(container: AppContainer, onChangeServer: () -> Unit) {
    val navController = rememberNavController()
    val backStack by navController.currentBackStackEntryAsState()
    val currentRoute = backStack?.destination?.route ?: "kasir"

    val user by container.session.user.collectAsState()
    val reachable by container.connectionWatcher.reachable.collectAsState()
    val checking by container.connectionWatcher.checking.collectAsState()

    val visibleTabs = TABS.filter { it.perm == null || container.session.can(it.perm) }
    val startRoute = visibleTabs.first().route

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        TABS.firstOrNull { it.route == currentRoute }?.let { tabTitle(it.route) }
                            ?: "GALAXYAS POS",
                    )
                },
                actions = {
                    Text(
                        user?.name ?: "",
                        style = MaterialTheme.typography.bodySmall,
                        modifier = Modifier.padding(end = 12.dp),
                        color = MaterialTheme.colorScheme.onPrimary,
                    )
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primary,
                    titleContentColor = MaterialTheme.colorScheme.onPrimary,
                ),
            )
        },
        bottomBar = {
            NavigationBar {
                visibleTabs.forEach { tab ->
                    NavigationBarItem(
                        selected = currentRoute == tab.route,
                        onClick = {
                            navController.navigate(tab.route) {
                                popUpTo(navController.graph.startDestinationId) { saveState = true }
                                launchSingleTop = true
                                restoreState = true
                            }
                        },
                        icon = { Text(tab.icon) },
                        label = { Text(tab.label) },
                    )
                }
            }
        },
    ) { inner ->
        Column(modifier = Modifier.padding(inner)) {
            if (!reachable) {
                ConnectionBanner(checking) { container.connectionWatcher.checkAsync() }
            }
            NavHost(navController = navController, startDestination = startRoute) {
                composable("kasir") {
                    ComingSoon("🧾", "Kasir", "Layar kasir lengkap hadir di tahap berikutnya (P2).")
                }
                composable("produk") {
                    val vm: ProductListViewModel = viewModel { ProductListViewModel(container.api) }
                    ProductListScreen(vm)
                }
                composable("laporan") {
                    ComingSoon("📊", "Laporan", "Laporan penjualan & persediaan hadir di P4.")
                }
                composable("menu") {
                    MenuScreen(
                        session = container.session,
                        registry = container.serverRegistry,
                        settings = container.settings,
                        onChangeServer = onChangeServer,
                    )
                }
            }
        }
    }
}

private fun tabTitle(route: String): String = when (route) {
    "kasir" -> "Kasir"
    "produk" -> "Data Barang"
    "laporan" -> "Laporan"
    else -> "Menu"
}
