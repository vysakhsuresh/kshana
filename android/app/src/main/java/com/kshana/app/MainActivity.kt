package com.kshana.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.List
import androidx.compose.material.icons.filled.Place
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.ProfileStore
import com.kshana.app.ui.screens.AreasScreen
import com.kshana.app.ui.screens.BirthScreen
import com.kshana.app.ui.screens.ChartScreen
import com.kshana.app.ui.screens.MomentsScreen
import com.kshana.app.ui.screens.MuhurtaScreen
import com.kshana.app.ui.screens.NowScreen
import com.kshana.app.ui.screens.PariharaScreen
import com.kshana.app.ui.screens.PrashnaScreen
import com.kshana.app.ui.screens.ReadingScreen
import com.kshana.app.ui.screens.SettingsScreen
import com.kshana.app.ui.screens.TrustScreen
import com.kshana.app.ui.screens.VedicScreen
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.KshanaTheme

private data class Tab(val route: String, val label: String, val icon: ImageVector)

private val tabs = listOf(
    Tab("now", "NOW", Icons.Filled.Star),
    Tab("chart", "CHART", Icons.Filled.Place),
    Tab("vedic", "VEDIC", Icons.Filled.List),
    Tab("moments", "MOMENTS", Icons.Filled.DateRange),
    Tab("reading", "READING", Icons.Filled.Info),
)

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent { KshanaTheme { App() } }
    }
}

@Composable
private fun App() {
    val context = LocalContext.current
    val nav = rememberNavController()
    var profile by remember { mutableStateOf(ProfileStore.load(context)) }
    var settings by remember { mutableStateOf(ProfileStore.settings(context)) }
    // First launch with no profile lands on the birth form; afterwards on Now.
    val start = remember { if (profile == null) "birth" else "now" }
    val backStack by nav.currentBackStackEntryAsState()
    val current = backStack?.destination?.route ?: start
    val showBar = tabs.any { it.route == current }

    fun goTab(route: String) {
        if (current == route) return
        nav.navigate(route) {
            launchSingleTop = true
            restoreState = true
            popUpTo(nav.graph.findStartDestination().id) { saveState = true }
        }
    }

    /** Screens that need a birth go to the form when there is none. */
    @Composable
    fun withProfile(content: @Composable (BirthProfile) -> Unit) {
        val p = profile
        if (p == null) {
            BirthScreen(existing = null, onCast = { profile = it; nav.navigate("chart") { launchSingleTop = true } })
        } else {
            content(p)
        }
    }

    Scaffold(
        containerColor = KshanaColors.Night,
        bottomBar = {
            if (showBar) {
                NavigationBar(containerColor = KshanaColors.Night) {
                    for (t in tabs) {
                        NavigationBarItem(
                            selected = current == t.route,
                            onClick = { goTab(t.route) },
                            icon = { Icon(t.icon, contentDescription = t.label) },
                            label = { Text(t.label) },
                            colors = NavigationBarItemDefaults.colors(
                                selectedIconColor = KshanaColors.Gold,
                                selectedTextColor = KshanaColors.Gold,
                                unselectedIconColor = KshanaColors.Muted,
                                unselectedTextColor = KshanaColors.Muted,
                                indicatorColor = KshanaColors.Surface,
                            ),
                        )
                    }
                }
            }
        },
    ) { padding ->
        NavHost(nav, startDestination = start, modifier = Modifier.padding(padding)) {
            composable("now") {
                NowScreen(
                    place = profile?.place() ?: ProfileStore.currentLocation(context),
                    settings = settings,
                    onAsk = { nav.navigate("prashna") { launchSingleTop = true } },
                    onSettings = { nav.navigate("settings") { launchSingleTop = true } },
                )
            }
            composable("chart") { withProfile { ChartScreen(it, settings) } }
            composable("vedic") { withProfile { VedicScreen(it, settings) } }
            composable("moments") {
                MomentsScreen(ayanamsa = settings.ayanamsa, onMuhurta = { nav.navigate("muhurta") { launchSingleTop = true } })
            }
            composable("muhurta") {
                MuhurtaScreen(
                    place = profile?.place() ?: ProfileStore.currentLocation(context),
                    settings = settings,
                    profile = profile,
                    onBack = { nav.popBackStack() },
                )
            }
            composable("reading") {
                withProfile {
                    ReadingScreen(
                        it, settings,
                        onAreas = { nav.navigate("areas") { launchSingleTop = true } },
                        onParihara = { nav.navigate("parihara") { launchSingleTop = true } },
                    )
                }
            }
            composable("areas") { withProfile { AreasScreen(it, settings, onBack = { nav.popBackStack() }) } }
            composable("parihara") { withProfile { PariharaScreen(it, settings, onBack = { nav.popBackStack() }) } }
            composable("prashna") {
                PrashnaScreen(
                    place = profile?.place() ?: ProfileStore.currentLocation(context),
                    settings = settings,
                    onBack = { nav.popBackStack() },
                )
            }
            composable("birth") {
                BirthScreen(
                    existing = profile,
                    onCast = { p ->
                        profile = p
                        nav.navigate("chart") {
                            launchSingleTop = true
                            popUpTo("birth") { inclusive = true }
                        }
                    },
                )
            }
            composable("settings") {
                SettingsScreen(
                    profile = profile,
                    settings = settings,
                    onSettingsChanged = { settings = it },
                    onEditBirth = { nav.navigate("birth") { launchSingleTop = true } },
                    onTrust = { nav.navigate("trust") { launchSingleTop = true } },
                    onBack = { nav.popBackStack() },
                )
            }
            composable("trust") {
                TrustScreen(profile = profile, settings = settings, onBack = { nav.popBackStack() })
            }
        }
    }
}
