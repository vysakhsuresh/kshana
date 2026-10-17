package com.kshana.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.Favorite
import androidx.compose.material.icons.filled.Info
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
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.ProfileStore
import com.kshana.app.ui.screens.AreasScreen
import com.kshana.app.ui.screens.AskScreen
import com.kshana.app.ui.screens.BirthScreen
import com.kshana.app.ui.screens.BookScreen
import com.kshana.app.ui.screens.ChartScreen
import com.kshana.app.ui.screens.HindsightScreen
import com.kshana.app.ui.screens.LetterScreen
import com.kshana.app.ui.screens.RecordScreen
import com.kshana.app.ui.screens.WallScreen
import com.kshana.app.ui.screens.PlanScreen
import com.kshana.app.ui.screens.RectifyScreen
import com.kshana.app.ui.screens.TogetherScreen
import com.kshana.app.ui.screens.LifeScreen
import com.kshana.app.ui.screens.LoopScreen
import com.kshana.app.ui.screens.MomentsScreen
import com.kshana.app.ui.screens.MoreScreen
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

/** The five doors: the loop, a question, a time to plan, the two of you, and everything else. */
private val tabs = listOf(
    Tab("loop", "LOOP", Icons.Filled.Star),
    Tab("prashna", "ASK", Icons.Filled.Info),
    Tab("plan", "PLAN", Icons.Filled.DateRange),
    Tab("together", "US", Icons.Filled.Favorite),
    Tab("more", "MORE", Icons.AutoMirrored.Filled.List),
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
    // The chosen language, and the sunrise alarm re-armed on every launch when it is on.
    remember {
        com.kshana.app.data.Lang.load(context)
        com.kshana.app.system.Alerts.rearm(context)
        if (com.kshana.app.system.Sunrise.enabled(context)) com.kshana.app.system.Sunrise.scheduleNext(context)
        true
    }
    var settings by remember { mutableStateOf(ProfileStore.settings(context)) }
    // First launch with no profile lands on the birth form; afterwards on the loop.
    val start = remember { if (profile == null) "birth" else "loop" }
    val backStack by nav.currentBackStackEntryAsState()
    val current = backStack?.destination?.route ?: start
    val showBar = tabs.any { it.route == current }
    // Where the person is now: a saved current place, else the birth place, else Kochi.
    var here by remember { mutableStateOf(ProfileStore.currentLocation(context)) }

    /**
     * Tabs sit on top of the Loop: every tab tap clears whatever was pushed above the Loop
     * and shows the tab. No saved state is restored, so a tab never comes back showing a
     * screen that was pushed over it earlier (the Loop button → Ask → Loop tab bug).
     */
    fun goTab(route: String) {
        if (current == route) return
        val hasLoop = runCatching { nav.getBackStackEntry("loop") }.isSuccess
        nav.navigate(route) {
            launchSingleTop = true
            if (hasLoop) popUpTo("loop") { inclusive = route == "loop" }
            else popUpTo(nav.graph.findStartDestination().id) { inclusive = true }
        }
    }
    fun open(route: String) = nav.navigate(route) { launchSingleTop = true }

    /** Screens that need a birth go to the form when there is none. */
    @Composable
    fun withProfile(content: @Composable (BirthProfile) -> Unit) {
        val p = profile
        if (p == null) {
            BirthScreen(existing = null, onCast = { profile = it; open("loop") })
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
                            label = { Text(com.kshana.app.data.Lang.t(t.label)) },
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
            composable("loop") {
                withProfile {
                    LoopScreen(
                        profile = it, place = here, settings = settings,
                        onLife = { life -> open("life/$life") },
                        onAsk = { open("prashna") },
                        onRemedy = { open("parihara") },
                        onSettings = { open("settings") },
                        onPlan = { goTab("plan") },
                        onBook = { open("book") },
                        onTogether = { goTab("together") },
                        onWall = { open("wall") },
                        onLetter = { open("letter") },
                        onHindsight = { open("hindsight") },
                        onRecord = { open("record") },
                    )
                }
            }
            composable("wall") { withProfile { WallScreen(profile = it, place = here, settings = settings, onBack = { nav.popBackStack() }) } }
            composable("letter") {
                withProfile { LetterScreen(profile = it, place = here, settings = settings, onBack = { nav.popBackStack() }, onLife = { life -> open("life/$life") }, onWall = { open("wall") }) }
            }
            composable("hindsight") {
                withProfile { HindsightScreen(profile = it, place = here, settings = settings, onBack = { nav.popBackStack() }, onLife = { life -> open("life/$life") }) }
            }
            composable("record") { RecordScreen(onBack = { nav.popBackStack() }, onLife = { life -> open("life/$life") }) }
            composable("plan") {
                withProfile { PlanScreen(profile = it, place = here, settings = settings, onLife = { life -> open("life/$life") }) }
            }
            composable("together") {
                withProfile {
                    TogetherScreen(profile = it, place = here, settings = settings, onAddPartner = { open("partner") }, onRemedy = { l -> open("parihara/$l") })
                }
            }
            composable("partner") {
                BirthScreen(
                    existing = null,
                    onCast = { p ->
                        ProfileStore.addPartner(context, p)
                        nav.popBackStack()
                    },
                    heading = "THE OTHER PERSON",
                    title = "Their moment of arrival",
                    intro = "Their date, clock time and place. If the time is unknown, enter 12:00 and set the slider to unknown; the Moon and the match still hold.",
                    nameLabel = "Their name",
                    button = "ADD TO US",
                    persist = false,
                    onBack = { nav.popBackStack() },
                )
            }
            composable("book") {
                withProfile { BookScreen(profile = it, place = here, settings = settings, onBack = { nav.popBackStack() }, onLife = { life -> open("life/$life") }) }
            }
            composable("rectify") {
                withProfile {
                    RectifyScreen(profile = it, settings = settings, onBack = { nav.popBackStack() }, onSaved = { p ->
                        profile = p
                        nav.popBackStack()
                    })
                }
            }
            composable("life/{life}", arguments = listOf(navArgument("life") { type = NavType.StringType })) { entry ->
                val life = entry.arguments?.getString("life") ?: "Love"
                withProfile {
                    LifeScreen(
                        profile = it, place = here, settings = settings, life = life,
                        onBack = { nav.popBackStack() },
                        onAsk = { open("ask/$life") },
                        onRemedy = { open("parihara/$life") },
                    )
                }
            }
            composable("more") { MoreScreen(profile = profile, onOpen = { open(it) }) }
            composable("now") {
                NowScreen(
                    place = here,
                    settings = settings,
                    onAsk = { open("prashna") },
                    onSettings = { open("settings") },
                )
            }
            composable("chart") { withProfile { ChartScreen(it, settings) } }
            composable("vedic") { withProfile { VedicScreen(it, settings) } }
            composable("moments") {
                MomentsScreen(ayanamsa = settings.ayanamsa, onMuhurta = { open("muhurta") })
            }
            composable("muhurta") {
                MuhurtaScreen(place = here, settings = settings, profile = profile, onBack = { nav.popBackStack() })
            }
            composable("reading") {
                withProfile {
                    ReadingScreen(it, settings, onAreas = { open("areas") }, onParihara = { open("parihara") })
                }
            }
            composable("areas") { withProfile { AreasScreen(it, settings, onBack = { nav.popBackStack() }) } }
            composable("parihara") { withProfile { PariharaScreen(it, settings, onBack = { nav.popBackStack() }) } }
            composable("parihara/{life}", arguments = listOf(navArgument("life") { type = NavType.StringType })) { entry ->
                val life = entry.arguments?.getString("life")
                withProfile { PariharaScreen(it, settings, onBack = { nav.popBackStack() }, life = life) }
            }
            composable("prashna") {
                withProfile {
                    AskScreen(profile = it, place = here, settings = settings, onRemedy = { l -> open("parihara/$l") })
                }
            }
            composable("ask/{life}", arguments = listOf(navArgument("life") { type = NavType.StringType })) { entry ->
                val life = entry.arguments?.getString("life")
                withProfile {
                    AskScreen(profile = it, place = here, settings = settings, initialLife = life, onBack = { nav.popBackStack() }, onRemedy = { l -> open("parihara/$l") })
                }
            }
            composable("prashna_expert") {
                PrashnaScreen(place = here, settings = settings, onBack = { nav.popBackStack() })
            }
            composable("birth") {
                BirthScreen(
                    existing = profile,
                    onCast = { p ->
                        profile = p
                        here = ProfileStore.currentLocation(context)
                        nav.navigate("loop") {
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
                    onEditBirth = { open("birth") },
                    onTrust = { open("trust") },
                    onBack = { nav.popBackStack() },
                    onPlaceChanged = { here = it },
                )
            }
            composable("trust") {
                TrustScreen(profile = profile, settings = settings, onBack = { nav.popBackStack() })
            }
        }
    }
}
