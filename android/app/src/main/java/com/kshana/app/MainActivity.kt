package com.kshana.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.automirrored.filled.ArrowForward
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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.background
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
import com.kshana.app.data.FamilyStore
import com.kshana.app.data.ProfileStore
import com.kshana.app.ui.screens.AreasScreen
import com.kshana.app.ui.screens.AheadScreen
import com.kshana.app.ui.screens.AskHub
import com.kshana.app.ui.screens.AskScreen
import com.kshana.app.ui.screens.BirthScreen
import com.kshana.app.ui.screens.BookScreen
import com.kshana.app.ui.screens.CalendarScreen
import com.kshana.app.ui.screens.ChartScreen
import com.kshana.app.ui.screens.CompatibilityScreen
import com.kshana.app.ui.screens.DiaryScreen
import com.kshana.app.ui.screens.FamilyScreen
import com.kshana.app.ui.screens.GlossaryScreen
import com.kshana.app.ui.screens.HindsightScreen
import com.kshana.app.ui.screens.LedgerScreen
import com.kshana.app.ui.screens.LetterScreen
import com.kshana.app.ui.screens.LifeMapScreen
import com.kshana.app.ui.screens.LifeScreen
import com.kshana.app.ui.screens.LoopScreen
import com.kshana.app.ui.screens.MeScreen
import com.kshana.app.ui.screens.PlanScreen
import com.kshana.app.ui.screens.RecordScreen
import com.kshana.app.ui.screens.TogetherScreen
import com.kshana.app.ui.screens.WallScreen
import com.kshana.app.ui.screens.MomentsScreen
import com.kshana.app.ui.screens.MoreScreen
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
import com.kshana.app.ui.screens.LexiconScreen
import com.kshana.app.ui.screens.MargaScreen

private data class Tab(val route: String, val label: String, val glyph: com.kshana.app.ui.theme.TabGlyph)

/** The six doors: today, your reading (with Ask inside it), the future, the two of you, your favorites, and everything else. */
private val tabs = listOf(
    Tab("loop", "NOW", com.kshana.app.ui.theme.TabGlyph.SUN),
    Tab("reading", "READING", com.kshana.app.ui.theme.TabGlyph.EYE),
    Tab("future", "FUTURE", com.kshana.app.ui.theme.TabGlyph.CRYSTAL),
    Tab("together", "US", com.kshana.app.ui.theme.TabGlyph.HEART),
    Tab("favorites", "FAVORITES", com.kshana.app.ui.theme.TabGlyph.STAR),
    Tab("more", "EXPLORE", com.kshana.app.ui.theme.TabGlyph.MORE),
)

/** A kshana:// link the app was opened with, waiting to be handled by the UI. */
internal object IncomingLink {
    var value by mutableStateOf<String?>(null)
}

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        IncomingLink.value = intent?.dataString
        KshanaFonts.attach(this)
        setContent { KshanaTheme { App() } }
    }

    override fun onNewIntent(intent: android.content.Intent) {
        super.onNewIntent(intent)
        IncomingLink.value = intent.dataString
    }

    override fun onStop() {
        super.onStop()
        // Keep the copy in Downloads current, off the main thread.
        val app = applicationContext
        Thread { com.kshana.app.data.Backup.autoSave(app) }.start()
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
        com.kshana.app.data.Focus.load(context)
        com.kshana.app.data.Situations.load(context)
        com.kshana.app.system.Alerts.rearm(context)
        if (com.kshana.app.system.Sunrise.enabled(context)) com.kshana.app.system.Sunrise.scheduleNext(context)
        true
    }
    var settings by remember { mutableStateOf(ProfileStore.settings(context)) }
    // Plain/simple mode for the five-door screens kept under More.
    var simple by remember { mutableStateOf(FamilyStore.simpleMode(context)) }
    // First launch with no profile lands on the welcome (start or restore), then the birth
    // form and the past check; afterwards on the loop. The arrival ritual stays registered.
    val start = remember { if (profile == null) "welcome" else "loop" }
    // A shared person opened from a kshana://person link: add them under Us and show them.
    androidx.compose.runtime.LaunchedEffect(IncomingLink.value, profile) {
        val link = IncomingLink.value ?: return@LaunchedEffect
        val p = com.kshana.app.data.PersonLink.parse(link) ?: run { IncomingLink.value = null; return@LaunchedEffect }
        if (profile == null) return@LaunchedEffect
        val added = com.kshana.app.data.PersonLink.add(context, p)
        IncomingLink.value = null
        android.widget.Toast.makeText(context, if (added) "Added ${p.name} under Us" else "${p.name} is already under Us", android.widget.Toast.LENGTH_LONG).show()
        nav.navigate("together") { launchSingleTop = true }
    }
    val backStack by nav.currentBackStackEntryAsState()
    val current = backStack?.destination?.route ?: start
    val showBar = tabs.any { it.route == current }
    // Which pages the person has opened: the Explore dot and the Now cards read it.
    androidx.compose.runtime.LaunchedEffect(profile) { if (profile != null) com.kshana.app.data.Discover.reload(context) }
    androidx.compose.runtime.LaunchedEffect(current, profile) { if (profile != null) com.kshana.app.data.Discover.visit(context, current) }
    val exploreDot = profile != null && com.kshana.app.ui.screens.MORE_PAGES.any {
        it.section in com.kshana.app.ui.screens.NEED_SECTIONS && com.kshana.app.data.Discover.isNew(it.route)
    }
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
                    here = ProfileStore.currentLocation(context)
                    // The first birth entry replaces the form in the back stack.
                    nav.navigate("loop") {
                        launchSingleTop = true
                        popUpTo(nav.graph.findStartDestination().id) { inclusive = true }
                    }
                },
            )
        } else {
            content(p)
        }
    }

    androidx.compose.foundation.layout.Box(Modifier.fillMaxSize()) {
    com.kshana.app.ui.theme.NightSky()
    Scaffold(
        containerColor = androidx.compose.ui.graphics.Color.Transparent,
        bottomBar = {
            if (showBar) {
                NavigationBar(containerColor = KshanaColors.Night.copy(alpha = 0.86f), tonalElevation = 0.dp) {
                    for (t in tabs) {
                        NavigationBarItem(
                            selected = current == t.route,
                            onClick = { goTab(t.route) },
                            icon = {
                                androidx.compose.foundation.layout.Box {
                                    com.kshana.app.ui.theme.AnimatedTabIcon(t.glyph, selected = current == t.route, label = t.label)
                                    // A small gold dot while Explore holds pages not yet opened.
                                    if (t.route == "more" && exploreDot) {
                                        androidx.compose.foundation.layout.Box(
                                            Modifier.align(androidx.compose.ui.Alignment.TopEnd)
                                                .size(7.dp)
                                                .background(KshanaColors.Gold, androidx.compose.foundation.shape.CircleShape),
                                        )
                                    }
                                }
                            },
                            label = { Text(com.kshana.app.data.Lang.t(t.label), fontSize = 10.sp, maxLines = 1, softWrap = false, letterSpacing = 0.6.sp) },
                            colors = NavigationBarItemDefaults.colors(
                                selectedIconColor = KshanaColors.Gold,
                                selectedTextColor = KshanaColors.Gold,
                                unselectedIconColor = KshanaColors.Muted,
                                unselectedTextColor = KshanaColors.Muted,
                                indicatorColor = KshanaColors.Gold.copy(alpha = 0.12f),
                            ),
                        )
                    }
                }
            }
        },
    ) { padding ->
        NavHost(
            nav, startDestination = start, modifier = Modifier.padding(padding),
            enterTransition = { androidx.compose.animation.fadeIn(androidx.compose.animation.core.tween(320)) },
            exitTransition = { androidx.compose.animation.fadeOut(androidx.compose.animation.core.tween(220)) },
            popEnterTransition = { androidx.compose.animation.fadeIn(androidx.compose.animation.core.tween(320)) },
            popExitTransition = { androidx.compose.animation.fadeOut(androidx.compose.animation.core.tween(220)) },
        ) {
            composable("loop") {
                withProfile {
                    LoopScreen(
                        profile = it, place = here, settings = settings,
                        onLife = { life -> open("life/$life") },
                        onAsk = { open("prashna") },
                        onRemedy = { open("parihara") },
                        onSettings = { open("settings") },
                        onPlan = { open("plan") },
                        onAhead = { com.kshana.app.ui.screens.FutureHubRequest.mode = "near"; goTab("future") },
                        onBook = { open("book") },
                        onTogether = { goTab("together") },
                        onWall = { open("wall") },
                        onLetter = { open("letter") },
                        onHindsight = { open("hindsight") },
                        onRecord = { open("record") },
                        onJournal = { open("journal") },
                        onCalendar = { open("calendar") },
                        onReading = { goTab("reading") },
                        onOpen = { r -> if (tabs.any { t -> t.route == r }) goTab(r) else open(r) },
                    )
                }
            }
            composable("welcome") {
                com.kshana.app.ui.screens.WelcomeScreen(
                    onStart = { open("birth") },
                    onRestored = {
                        profile = ProfileStore.load(context)
                        settings = ProfileStore.settings(context)
                        here = ProfileStore.currentLocation(context)
                        nav.navigate("loop") { launchSingleTop = true; popUpTo("welcome") { inclusive = true } }
                    },
                )
            }
            composable("journal") {
                withProfile {
                    com.kshana.app.ui.screens.JournalScreen(
                        profile = it, place = here, settings = settings, onBack = { nav.popBackStack() },
                        onRectify = { open("rectify") },
                        onDay = { d -> com.kshana.app.ui.screens.HindsightRequest.date = d; open("hindsight") },
                    )
                }
            }
            composable("how") { com.kshana.app.ui.screens.HowScreen(onBack = { nav.popBackStack() }) }
            composable("familydates") { withProfile { com.kshana.app.ui.screens.FamilyDatesScreen(profile = it, place = here, settings = settings, onBack = { nav.popBackStack() }) } }
            composable("future") {
                withProfile {
                    com.kshana.app.ui.screens.FutureHub(
                        profile = it, place = here, settings = settings,
                        onLife = { life -> open("life/$life") }, onRemedy = { l -> open(l?.let { "parihara/$it" } ?: "parihara") },
                        onLetter = { open("letter") }, onBook = { open("book") }, onHindsight = { open("hindsight") }, onSettings = { open("settings") },
                        onWall = { open("wall") },
                    )
                }
            }
            composable("reading") {
                withProfile {
                    com.kshana.app.ui.screens.ConsultScreen(
                        profile = it, place = here, settings = settings,
                        doors = com.kshana.app.ui.screens.ReadingDoors(
                            ask = { open("prashna") },
                            remedy = { l -> open(l?.let { "parihara/$it" } ?: "parihara") },
                            future = { com.kshana.app.ui.screens.FutureHubRequest.mode = "far"; goTab("future") },
                            ahead = { com.kshana.app.ui.screens.FutureHubRequest.mode = "near"; goTab("future") },
                            today = { goTab("loop") },
                            book = { open("book") },
                            record = { open("record") },
                            journal = { open("journal") },
                            how = { open("how") },
                            life = { l -> open("life/$l") },
                        ),
                    )
                }
            }
            composable("calendar") { withProfile { com.kshana.app.ui.screens.CalendarScreen(profile = it, place = here, settings = settings, onBack = { nav.popBackStack() }, onRemedy = { open("parihara") }) } }
            composable("wall") { withProfile { WallScreen(profile = it, place = here, settings = settings, onBack = { nav.popBackStack() }, onRemedy = { open("parihara") }) } }
            composable("letter") {
                withProfile { LetterScreen(profile = it, place = here, settings = settings, onBack = { nav.popBackStack() }, onLife = { life -> open("life/$life") }, onWall = { open("wall") }) }
            }
            composable("hindsight") {
                withProfile { HindsightScreen(profile = it, place = here, settings = settings, onBack = { nav.popBackStack() }, onLife = { life -> open("life/$life") }) }
            }
            composable("record") { RecordScreen(onBack = { nav.popBackStack() }, onLife = { life -> open("life/$life") }) }
            composable("plan") {
                withProfile { AskHub(profile = it, place = here, settings = settings, initialMode = "time", onRemedy = { l -> open("parihara/$l") }, onLife = { life -> open("life/$life") }) }
            }
            composable("ahead") {
                withProfile {
                    AheadScreen(
                        profile = it, place = here, settings = settings,
                        onLife = { life -> open("life/$life") },
                        onRemedy = { l -> open("parihara/$l") },
                        onLetter = { open("letter") },
                        onBook = { open("book") },
                        onHindsight = { open("hindsight") },
                    )
                }
            }
            composable("together") {
                withProfile {
                    TogetherScreen(profile = it, place = here, settings = settings, onAddPartner = { open("partner") }, onRemedy = { l -> open("parihara/$l") }, onFamilyDates = { open("familydates") })
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
                    intro = "Their date, clock time and place. If the time is unknown, enter 12:00 and set the slider to unknown; the match and the broad readings still hold.",
                    nameLabel = "Their name",
                    button = "ADD TO US",
                    persist = false,
                    onBack = { nav.popBackStack() },
                )
            }
            composable("book") {
                withProfile { BookScreen(profile = it, place = here, settings = settings, onBack = { nav.popBackStack() }, onLife = { life -> open("life/$life") }, onRemedy = { open("parihara") }) }
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
            composable("favorites") { com.kshana.app.ui.screens.FavoritesScreen(onOpen = { r -> if (tabs.any { it.route == r }) goTab(r) else open(r) }) }
            composable("guide") { com.kshana.app.ui.screens.GuideScreen(onBack = back, onOpen = { r -> if (tabs.any { it.route == r }) goTab(r) else open(r) }) }
            composable("about") { com.kshana.app.ui.screens.AboutScreen(onBack = back, onOpen = { open(it) }) }

            // ---- the five-door screens from the overnight build, kept under More ----
            // (reached from More behind the "Show the chart pages" setting; not tabs)
            composable("today") {
                withProfile {
                    LivingSky(place = here, settings = settings, intensity = 0.7f) {
                        TodayScreen(
                            profile = it,
                            here = here,
                            settings = settings,
                            simple = simple,
                            onAsk = { open("prashna") },
                            onRightTime = { open("righttime") },
                            onLife = { open("mylife") },
                            onRiver = { open("trace") },
                            onSettings = { open("settings") },
                        )
                    }
                }
            }
            composable("mylife") {
                withProfile {
                    LifeScreen(
                        profile = it,
                        settings = settings,
                        simple = simple,
                        onLifeMap = { open("lifemap") },
                        onRiver = { open("trace") },
                        onAreas = { open("areas") },
                        onParihara = { open("parihara_classic") },
                        onChart = { open("reading_expert") },
                    )
                }
            }
            composable("righttime") {
                RightTimeScreen(
                    profile = profile,
                    here = here,
                    settings = settings,
                    onMuhurta = { open("muhurta") },
                    onCalendar = { open("skycalendar") },
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
                    onLexicon = { open("lexicon") },
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
                    onMarga = { open("marga") },
                )
            }

            // ---- behind those doors ----
            composable("lifemap") { withProfile { LifeMapScreen(it, settings, onBack = back) } }
            composable("transits") { withProfile { TransitsScreen(it, settings, onBack = back) } }
            composable("ledger") { withProfile { LedgerScreen(it, settings, onBack = back) } }
            composable("year") { withProfile { YearScreen(it, here, settings, onBack = back) } }
            composable("timelords") { withProfile { TimeLordsScreen(it, settings, onBack = back) } }
            composable("trace") { withProfile { TraceScreen(it, here, settings, onBack = back) } }
            composable("family") { FamilyScreen(onBack = back) }
            composable("compatibility") {
                CompatibilityScreen(settings = settings, onFamily = { open("family") }, onBack = back)
            }
            composable("naming") { NamingScreen(settings = settings, onBack = back) }
            composable("glossary") { GlossaryScreen(onBack = back) }
            // The observances-and-eclipses calendar; "calendar" is the person's own week check.
            composable("skycalendar") { CalendarScreen(here = here, settings = settings, onBack = back) }
            composable("narrate") {
                withProfile { NarrateScreen(it, settings, onBack = back, onSettings = { open("settings") }) }
            }
            composable("strength") { withProfile { StrengthScreen(it, settings, onBack = back) } }
            composable("lexicon") { LexiconScreen(onBack = back) }
            composable("marga") { withProfile { MargaScreen(it, here, settings, onBack = back) } }
            composable("sealed") { withProfile { SealedScreen(it, settings, onBack = back) } }
            composable("quiz") { withProfile { QuizScreen(it, settings, onBack = back) } }
            composable("day") { DayScreen(profile = profile, here = here, settings = settings, onBack = back) }
            composable("diary") { withProfile { DiaryScreen(it, here, settings, onBack = back) } }
            // The arrival ritual: an alternative first run. It saves the profile itself;
            // a first birth then goes through the past check like the birth form.
            composable("arrival") {
                ArrivalScreen(onDone = { p ->
                    profile = p
                    here = ProfileStore.currentLocation(context)
                    nav.navigate("pastcheck") {
                        launchSingleTop = true
                        popUpTo("arrival") { inclusive = true }
                    }
                })
            }

            // ---- the instrument ----
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
                MuhurtaScreen(place = here, settings = settings, profile = profile, onBack = back)
            }
            composable("reading_expert") {
                withProfile {
                    ReadingScreen(it, settings, onAreas = { open("areas") }, onParihara = { open("parihara") })
                }
            }
            composable("areas") { withProfile { AreasScreen(it, settings, onBack = { nav.popBackStack() }) } }
            composable("gifts") { withProfile { com.kshana.app.ui.screens.GiftsScreen(it, settings, onBack = back, onRemedy = { open("parihara") }) } }
            composable("almanac") { com.kshana.app.ui.screens.AlmanacScreen(here, settings, onBack = back) }
            composable("kpfull") { withProfile { com.kshana.app.ui.screens.KpFullScreen(it, here, settings, onBack = back) } }
            composable("dashasmore") { withProfile { com.kshana.app.ui.screens.DashasMoreScreen(it, settings, onBack = back) } }
            composable("parihara") { withProfile { com.kshana.app.ui.screens.RemedyTreeScreen(profile = it, place = here, settings = settings, onBack = { nav.popBackStack() }) } }
            composable("parihara_classic") { withProfile { PariharaScreen(it, settings, onBack = { nav.popBackStack() }) } }
            composable("parihara/{life}", arguments = listOf(navArgument("life") { type = NavType.StringType })) { entry ->
                val life = entry.arguments?.getString("life")
                withProfile { com.kshana.app.ui.screens.RemedyTreeScreen(profile = it, place = here, settings = settings, onBack = { nav.popBackStack() }, life = life) }
            }
            composable("prashna") {
                withProfile {
                    AskHub(profile = it, place = here, settings = settings, onRemedy = { l -> open("parihara/$l") }, onLife = { life -> open("life/$life") }, onBack = { nav.popBackStack() }, onGo = { a ->
                        when (a.kind) {
                            "life" -> open("life/${a.life ?: "Love"}")
                            "outlook" -> { com.kshana.app.ui.screens.AheadRequest.span = a.detail; com.kshana.app.ui.screens.FutureHubRequest.mode = "near"; goTab("future") }
                            "future" -> { com.kshana.app.ui.screens.FutureHubRequest.mode = "far"; goTab("future") }
                            "remedy" -> open(a.life?.let { "parihara/$it" } ?: "parihara")
                            "fear" -> { if (!com.kshana.app.data.Situations.has(com.kshana.app.data.Situations.CHART_FEAR)) com.kshana.app.data.Situations.toggle(context, com.kshana.app.data.Situations.CHART_FEAR); goTab("reading") }
                            "us" -> goTab("together")
                            "plan" -> { com.kshana.app.ui.screens.PlanPrefill.purpose = a.detail; open("plan") }
                            "past" -> open("hindsight")
                        }
                    })
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
                        val first = profile == null
                        profile = p
                        here = ProfileStore.currentLocation(context)
                        // A first birth goes through the past check before the first reading.
                        nav.navigate(if (first) "pastcheck" else "loop") {
                            launchSingleTop = true
                            popUpTo("birth") { inclusive = true }
                        }
                    },
                )
            }
            composable("pastcheck") {
                withProfile {
                    com.kshana.app.ui.screens.PastCheckScreen(
                        profile = it, settings = settings,
                        onDone = { nav.navigate("loop") { launchSingleTop = true; popUpTo(nav.graph.findStartDestination().id) { inclusive = true } } },
                        onProfile = { p -> profile = p },
                        onRemedy = { open("parihara") },
                    )
                }
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
                    onLexicon = { open("lexicon") },
                    onBack = back,
                    onPlaceChanged = { here = it },
                )
            }
            composable("trust") {
                TrustScreen(profile = profile, settings = settings, onBack = back)
            }
        }
    }
    }
}
