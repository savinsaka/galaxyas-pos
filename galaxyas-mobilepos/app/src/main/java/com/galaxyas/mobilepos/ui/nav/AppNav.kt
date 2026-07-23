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
import com.galaxyas.mobilepos.ui.kasir.HistoryViewModel
import com.galaxyas.mobilepos.ui.kasir.KasirScreen
import com.galaxyas.mobilepos.ui.kasir.KasirViewModel
import com.galaxyas.mobilepos.ui.kasir.TransactionHistoryScreen
import com.galaxyas.mobilepos.ui.login.LoginScreen
import com.galaxyas.mobilepos.ui.laporan.ReportsScreen
import com.galaxyas.mobilepos.ui.menu.MenuScreen
import com.galaxyas.mobilepos.ui.menu.SettingsPrinterScreen
import com.galaxyas.mobilepos.ui.menu.SettingsStoreScreen
import com.galaxyas.mobilepos.ui.menu.UsersScreen
import com.galaxyas.mobilepos.ui.onboarding.PairingScreen
import com.galaxyas.mobilepos.ui.persediaan.ExpensesScreen
import com.galaxyas.mobilepos.ui.persediaan.OpnameScreen
import com.galaxyas.mobilepos.ui.persediaan.StockBatchScreen
import com.galaxyas.mobilepos.ui.produk.BrandsScreen
import com.galaxyas.mobilepos.ui.produk.CustomersScreen
import com.galaxyas.mobilepos.ui.produk.DataSheetScreen
import com.galaxyas.mobilepos.ui.produk.DiscountsScreen
import com.galaxyas.mobilepos.ui.produk.ProductEditScreen
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
                    if (currentRoute == "kasir") {
                        androidx.compose.material3.TextButton(
                            onClick = { navController.navigate("riwayat") },
                        ) {
                            Text("Riwayat", color = MaterialTheme.colorScheme.onPrimary)
                        }
                    }
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
                    val vm: KasirViewModel = viewModel {
                        KasirViewModel(
                            container.api, container.session, container.settings,
                            container.pendingSales, container.appContext,
                        )
                    }
                    KasirScreen(vm, container.api)
                }
                composable("riwayat") {
                    val vm: HistoryViewModel = viewModel {
                        HistoryViewModel(container.api, container.session, container.settings, container.appContext)
                    }
                    TransactionHistoryScreen(vm)
                }
                composable("produk") {
                    val vm: ProductListViewModel = viewModel { ProductListViewModel(container.api) }
                    ProductListScreen(
                        vm = vm,
                        onEdit = { p ->
                            container.buffer.product = p
                            navController.navigate("produk/edit")
                        },
                        onAdd = {
                            container.buffer.product = null
                            navController.navigate("produk/edit")
                        },
                        onOpen = { navController.navigate(it) },
                    )
                }
                composable("produk/edit") {
                    ProductEditScreen(
                        api = container.api,
                        initial = container.buffer.product,
                        onSaved = { navController.popBackStack() },
                    )
                }
                composable("merek") { BrandsScreen(container.api) }
                composable("diskon") { DiscountsScreen(container.api) }
                composable("pelanggan") { CustomersScreen(container.api) }
                composable("datasheet") { DataSheetScreen(container.api) }
                composable("opname") { OpnameScreen(container.api, container.session) }
                composable("batch") {
                    StockBatchScreen(container.api, container.session, container.settings)
                }
                composable("pengeluaran") { ExpensesScreen(container.api, container.session) }
                composable("laporan") {
                    ReportsScreen(container.api, container.settings)
                }
                composable("toko") { SettingsStoreScreen(container.settings) }
                composable("users") { UsersScreen(container.api, container.session) }
                composable("menu") {
                    MenuScreen(
                        session = container.session,
                        registry = container.serverRegistry,
                        settings = container.settings,
                        onChangeServer = onChangeServer,
                        onOpenPrinter = { navController.navigate("printer") },
                        onOpen = { navController.navigate(it) },
                    )
                }
                composable("printer") {
                    SettingsPrinterScreen(container.settings)
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
