package com.kshana.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Person
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
import com.kshana.app.data.FamilyStore
import com.kshana.app.data.ProfileStore
import com.kshana.app.ui.screens.AreasScreen
import com.kshana.app.ui.screens.BirthScreen
import com.kshana.app.ui.screens.CalendarScreen
import com.kshana.app.ui.screens.ChartScreen
import com.kshana.app.ui.screens.CompatibilityScreen
import com.kshana.app.ui.screens.DiaryScreen
import com.kshana.app.ui.screens.FamilyScreen
import com.kshana.app.ui.screens.GlossaryScreen
import com.kshana.app.ui.screens.LifeMapScreen
import com.kshana.app.ui.screens.LedgerScreen
import com.kshana.app.ui.screens.LifeScreen
import com.kshana.app.ui.screens.MeScreen
import com.kshana.app.ui.screens.MomentsScreen
import com.kshana.app.ui.screens.MuhurtaScreen
import com.kshana.app.ui.screens.NamingScreen
import com.kshana.app.ui.screens.NarrateScreen
import com.kshana.app.ui.screens.NowScreen
import com.kshana.app.ui.screens.PariharaScreen
import com.kshana.app.ui.screens.PrashnaScreen
import com.kshana.app.ui.screens.ReadingScreen
import com.kshana.app.ui.screens.RectifyScreen
import com.kshana.app.ui.screens.RightTimeScreen
import com.kshana.app.ui.screens.SettingsScreen
import com.kshana.app.ui.screens.TimeLordsScreen
import com.kshana.app.ui.screens.TodayScreen
import com.kshana.app.ui.screens.TraceScreen
import com.kshana.app.ui.screens.TransitsScreen
import com.kshana.app.ui.screens.TrustScreen
import com.kshana.app.ui.screens.VedicScreen
import com.kshana.app.ui.screens.YearScreen
import com.kshana.app.ui.onboarding.ArrivalScreen
import com.kshana.app.ui.sky.LivingSky
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.KshanaTheme
import com.kshana.app.ui.screens.DayScreen
import com.kshana.app.ui.screens.StrengthScreen
import com.kshana.app.ui.sky.KshanaFonts
import com.kshana.app.ui.screens.SealedScreen
import com.kshana.app.ui.screens.QuizScreen

private data class Tab(val route: String, val label: String, val icon: ImageVector)

/** The five doors. Everything else is reached from one of them. */
private val tabs = listOf(
    Tab("today", "TODAY", Icons.Filled.Star),
    Tab("life", "MY LIFE", Icons.Filled.Info),
    Tab("prashna", "ASK", Icons.Filled.Place),
    Tab("righttime", "RIGHT TIME", Icons.Filled.DateRange),
    Tab("me", "ME", Icons.Filled.Person),
)

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        KshanaFonts.attach(this)
        setContent { KshanaTheme { App() } }
    }
}

@Composable
private fun App() {
    val context = LocalContext.current
    val nav = rememberNavController()
    var profile by remember { mutableStateOf(ProfileStore.load(context)) }
    var settings by remember { mutableStateOf(ProfileStore.settings(context)) }
    var simple by remember { mutableStateOf(FamilyStore.simpleMode(context)) }
    // First launch with no profile lands on the arrival ritual; afterwards on Today.
    val start = remember { if (profile == null) "arrival" else "today" }
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

    fun open(route: String) = nav.navigate(route) { launchSingleTop = true }
    val back: () -> Unit = { nav.popBackStack() }

    /** Screens that need a birth go to the form when there is none. */
    @Composable
    fun withProfile(content: @Composable (BirthProfile) -> Unit) {
        val p = profile
        if (p == null) {
            BirthScreen(
                existing = null,
                onCast = {
                    profile = it
                    nav.navigate("today") {
                        launchSingleTop = true
                        popUpTo(nav.graph.findStartDestination().id) { inclusive = true }
                    }
                },
            )
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
            // ---- the five doors ----
            composable("today") {
                withProfile {
                    val here = ProfileStore.currentLocation(context)
                    LivingSky(place = here, settings = settings, intensity = 0.7f) {
                        TodayScreen(
                            profile = it,
                            here = here,
                            settings = settings,
                            simple = simple,
                            onAsk = { goTab("prashna") },
                            onRightTime = { goTab("righttime") },
                            onLife = { goTab("life") },
                            onRiver = { open("trace") },
                            onSettings = { open("settings") },
                        )
                    }
                }
            }
            composable("life") {
                withProfile {
                    LifeScreen(
                        profile = it,
                        settings = settings,
                        simple = simple,
                        onLifeMap = { open("lifemap") },
                        onRiver = { open("trace") },
                        onAreas = { open("areas") },
                        onParihara = { open("parihara") },
                        onChart = { open("reading") },
                    )
                }
            }
            composable("prashna") {
                PrashnaScreen(
                    place = profile?.place() ?: ProfileStore.currentLocation(context),
                    settings = settings,
                    onBack = { goTab("today") },
                )
            }
            composable("righttime") {
                RightTimeScreen(
                    profile = profile,
                    here = ProfileStore.currentLocation(context),
                    settings = settings,
                    onMuhurta = { open("muhurta") },
                    onCalendar = { open("calendar") },
                    onMoments = { open("moments") },
                    onDay = { open("day") },
                )
            }
            composable("me") {
                MeScreen(
                    profile = profile,
                    settings = settings,
                    simple = simple,
                    onSimpleChanged = { simple = it; FamilyStore.saveSimpleMode(context, it) },
                    onEditBirth = { open("birth") },
                    onFamily = { open("family") },
                    onCompatibility = { open("compatibility") },
                    onNaming = { open("naming") },
                    onGlossary = { open("glossary") },
                    onChart = { open("chart") },
                    onVedic = { open("vedic") },
                    onTrust = { open("trust") },
                    onRectify = { open("rectify") },
                    onSettings = { open("settings") },
                    onNarrate = { open("narrate") },
                    onTransits = { open("transits") },
                    onRiver = { open("trace") },
                    onLedger = { open("ledger") },
                    onYear = { open("year") },
                    onTimeLords = { open("timelords") },
                    onDiary = { open("diary") },
                    onStrength = { open("strength") },
                    onSealed = { open("sealed") },
                    onQuiz = { open("quiz") },
                )
            }

            // ---- behind the doors ----
            composable("lifemap") { withProfile { LifeMapScreen(it, settings, onBack = back) } }
            composable("transits") { withProfile { TransitsScreen(it, settings, onBack = back) } }
            composable("ledger") { withProfile { LedgerScreen(it, settings, onBack = back) } }
            composable("year") {
                withProfile { YearScreen(it, ProfileStore.currentLocation(context), settings, onBack = back) }
            }
            composable("timelords") { withProfile { TimeLordsScreen(it, settings, onBack = back) } }
            composable("trace") {
                withProfile { TraceScreen(it, ProfileStore.currentLocation(context), settings, onBack = back) }
            }
            composable("family") { FamilyScreen(onBack = back) }
            composable("compatibility") {
                CompatibilityScreen(settings = settings, onFamily = { open("family") }, onBack = back)
            }
            composable("naming") { NamingScreen(settings = settings, onBack = back) }
            composable("glossary") { GlossaryScreen(onBack = back) }
            composable("calendar") {
                CalendarScreen(here = ProfileStore.currentLocation(context), settings = settings, onBack = back)
            }
            composable("narrate") {
                withProfile { NarrateScreen(it, settings, onBack = back, onSettings = { open("settings") }) }
            }

            // ---- the instrument ----
            composable("now") {
                NowScreen(
                    place = profile?.place() ?: ProfileStore.currentLocation(context),
                    settings = settings,
                    onAsk = { goTab("prashna") },
                    onSettings = { open("settings") },
                )
            }
            composable("chart") { withProfile { ChartScreen(it, settings) } }
            composable("vedic") { withProfile { VedicScreen(it, settings) } }
            composable("moments") {
                MomentsScreen(ayanamsa = settings.ayanamsa, onMuhurta = { open("muhurta") })
            }
            composable("muhurta") {
                MuhurtaScreen(
                    place = profile?.place() ?: ProfileStore.currentLocation(context),
                    settings = settings,
                    profile = profile,
                    onBack = back,
                )
            }
            composable("reading") {
                withProfile {
                    ReadingScreen(
                        it, settings,
                        onAreas = { open("areas") },
                        onParihara = { open("parihara") },
                    )
                }
            }
            composable("areas") { withProfile { AreasScreen(it, settings, onBack = back) } }
            composable("parihara") { withProfile { PariharaScreen(it, settings, onBack = back) } }
            composable("strength") { withProfile { StrengthScreen(it, settings, onBack = back) } }
            composable("sealed") { withProfile { SealedScreen(it, settings, onBack = back) } }
            composable("quiz") { withProfile { QuizScreen(it, settings, onBack = back) } }
            composable("day") {
                DayScreen(profile = profile, here = ProfileStore.currentLocation(context), settings = settings, onBack = back)
            }
            composable("rectify") {
                withProfile {
                    RectifyScreen(
                        it, settings,
                        onSaved = { p -> profile = p; nav.popBackStack() },
                        onBack = back,
                    )
                }
            }
            composable("arrival") {
                ArrivalScreen(onDone = { p ->
                    profile = p
                    nav.navigate("today") {
                        launchSingleTop = true
                        popUpTo("arrival") { inclusive = true }
                    }
                })
            }
            composable("diary") {
                withProfile { DiaryScreen(it, ProfileStore.currentLocation(context), settings, onBack = back) }
            }
            composable("birth") {
                BirthScreen(
                    existing = profile,
                    onCast = { p ->
                        profile = p
                        nav.navigate("today") {
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
                    onRectify = { open("rectify") },
                    onTrust = { open("trust") },
                    onNarrate = { open("narrate") },
                    onBack = back,
                )
            }
            composable("trust") {
                TrustScreen(profile = profile, settings = settings, onBack = back)
            }
        }
    }
}
