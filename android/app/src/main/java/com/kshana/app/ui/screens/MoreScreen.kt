package com.kshana.app.ui.screens

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.clickable
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.scale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.Favorites
import com.kshana.app.ui.theme.KshanaColors

/** One page reachable from More. [expert] pages show only with "Show the chart pages" on. */
internal data class MorePage(val section: String, val title: String, val detail: String, val route: String, val expert: Boolean = false)

private const val S_WORRY = "Something weighs on me"
private const val S_PLAN = "I have a day to plan"
private const val S_WAIT = "I am waiting for something"
private const val S_STORY = "My life so far"
private const val S_FAMILY = "For my family"

/** The sections grouped by what the person needs; their unopened pages light the Explore dot. */
internal val NEED_SECTIONS = setOf(S_WORRY, S_PLAN, S_WAIT, S_STORY, S_FAMILY)

/** One plain line under each need, saying when to come here. */
private val SECTION_LINES = mapOf(
    S_WORRY to "When a part of life feels heavy. Every hard stretch has something you can do.",
    S_PLAN to "An interview, a journey, a signing, a meeting: pick the hour that serves it.",
    S_WAIT to "A result, a birthday, a better season: see how far away it is.",
    S_STORY to "Look back, and see how often the reading was right for you.",
    S_FAMILY to "When many people must agree on one day.",
)

/** Every page under Explore, in the order shown. The Favorites tab reads the same list. */
internal val MORE_PAGES = listOf(
    MorePage(S_WORRY, "What helps", "deeds, giving and prayer for what weighs on you now, and why each helps", "parihara"),
    MorePage(S_WORRY, "Ask a question", "will it work out, and when: about anything on your mind", "prashna"),

    MorePage(S_PLAN, "Plan a day out", "say what you are going for; it finds the best hour", "plan"),
    MorePage(S_PLAN, "Your week's appointments", "your calendar, checked; a weak slot gets a better one", "calendar"),
    MorePage(S_PLAN, "The month's letter", "one calm page for the thirty days ahead", "letter"),
    MorePage(S_PLAN, "Calendar and festivals", "today in the Malayalam, Tamil and lunar calendars, and the festivals ahead where you are", "almanac"),

    MorePage(S_WAIT, "What you are waiting for", "every date that matters, counting down", "wall"),

    MorePage(S_STORY, "Your gifts", "the strengths and tender spots you were born with, and when each comes forward", "gifts"),
    MorePage(S_STORY, "The book of your life", "the years behind and ahead, as chapters", "book"),
    MorePage(S_STORY, "Look back at a day", "any day of your life, read as it was", "hindsight"),
    MorePage(S_STORY, "Your past, read", "dated stretches of your life from your birth time alone; every answer sharpens your birth time", "pastcheck"),
    MorePage(S_STORY, "Your journal", "write down what happened; see how often the app was right", "journal"),
    MorePage(S_STORY, "Did it happen?", "say whether the app was right; it keeps score", "record"),

    MorePage(S_FAMILY, "A date for all of us", "a wedding, a house-warming, a trip: the days that suit everyone", "familydates"),

    MorePage("You", "Birth details", "date, time, place and how sure you are", "birth"),
    MorePage("You", "Find your exact birth time", "work it out from things that happened in your life", "rectify"),
    MorePage("You", "Settings", "where you are, language, the sunrise reading", "settings"),

    MorePage("Help", "Guide", "how to use Kshana, screen by screen", "guide"),
    MorePage("Help", "How Kshana works", "what it reads, how sure it is, your data, privacy", "how"),
    MorePage("Help", "About Kshana", "who we are, what the name means, the version", "about"),

    MorePage("Chart pages", "Why trust this", "the engine, its data and its checks", "trust", true),
    MorePage("Chart pages", "The sky now", "live wheel, ascendant, panchanga", "now", true),
    MorePage("Chart pages", "Remedies, classic list", "every remedy for the running periods, with sources", "parihara_classic", true),
    MorePage("Chart pages", "Your birth chart", "Western wheel and aspects", "chart", true),
    MorePage("Chart pages", "Vedic chart", "rasi, vargas, Vimshottari, Ashtakavarga", "vedic", true),
    MorePage("Chart pages", "KP, in full", "sub-sub lords, four-level significators, cuspal interlinks, ruling planets", "kpfull", true),
    MorePage("Chart pages", "Every period system", "Narayana, Sthira, Shoola, Drig and the conditional dashas", "dashasmore", true),
    MorePage("Chart pages", "Life areas, in full", "KP promise and Parashari assessment with every rule", "areas", true),
    MorePage("Chart pages", "Strongest area, with rules", "the strongest area now with its rule trace", "reading_expert", true),
    MorePage("Chart pages", "Exact moments", "ingresses, stations, lunations", "moments", true),
    MorePage("Chart pages", "Muhurta", "the best windows for a purpose", "muhurta", true),
    MorePage("Chart pages", "Prashna, horary", "the KP horary chart with a number, for the expert", "prashna_expert", true),

    MorePage("The sky, day by day", "Today under the sky", "the living sky, the good hours and what the day holds", "today", true),
    MorePage("The sky, day by day", "The right time", "good-hours bar, muhurta, observances, the day's clocks", "righttime", true),
    MorePage("The sky, day by day", "The day's clocks", "sunrise, hora, choghadiya and the day's divisions", "day", true),
    MorePage("The sky, day by day", "Observances and eclipses", "Ekadashi, Pradosha, lunations, Sankranti, eclipses", "skycalendar", true),
    MorePage("The sky, day by day", "Transits", "gochara over the natal chart, scored", "transits", true),

    MorePage("The life, traced", "My life, the reading", "the plain reading by sections, with the rules behind it", "mylife", true),
    MorePage("The life, traced", "The life map", "a hundred years of periods, area by area", "lifemap", true),
    MorePage("The life, traced", "The river", "the life trace: every exact instant", "trace", true),
    MorePage("The life, traced", "The ledger", "what the chart said, against what you logged", "ledger", true),
    MorePage("The life, traced", "The year", "Tajika varshaphala for the running year", "year", true),
    MorePage("The life, traced", "Time lords", "further dashas and Western directions", "timelords", true),
    MorePage("The life, traced", "Strength", "Shadbala and the planets' weights", "strength", true),
    MorePage("The life, traced", "Sealed predictions", "predictions sealed now, opened later", "sealed", true),
    MorePage("The life, traced", "The quiz", "test the reading against your past", "quiz", true),
    MorePage("The life, traced", "Prasna Marga", "the Kerala horary method", "marga", true),
    MorePage("The life, traced", "Diary", "the day's sky beside what you wrote", "diary", true),

    MorePage("People and words", "Me", "your profile and its doors", "me", true),
    MorePage("People and words", "Family", "the people you keep charts for", "family", true),
    MorePage("People and words", "Compatibility", "kuta matching between two charts", "compatibility", true),
    MorePage("People and words", "Baby name syllable", "the starting syllable from the birth star", "naming", true),
    MorePage("People and words", "Glossary", "the terms, explained", "glossary", true),
    MorePage("People and words", "Lexicon", "the names of stars, tithis and grahas in your script", "lexicon", true),
    MorePage("People and words", "Narration", "an optional reading in prose, with your own key", "narrate", true),
    MorePage("People and words", "The arrival", "start again with the sky (asks for your birth again)", "arrival", true),
)

/** Explore: every page beyond the tabs, grouped by what the person needs, then their details, help and the chart pages. */
@Composable
fun MoreScreen(profile: BirthProfile?, onOpen: (String) -> Unit) {
    val context = LocalContext.current
    LaunchedEffect(Unit) { Favorites.reload(context) }
    val expert = com.kshana.app.data.Expert.on(context)
    val open: (String) -> Unit = { Favorites.opened(context, it); onOpen(it) }
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("EXPLORE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(profile?.name?.takeIf { it.isNotBlank() } ?: "You", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        if (profile != null) {
            Text(
                "%d %s %d · %02d:%02d · %s".format(profile.day, MONTHS[(profile.month - 1).coerceIn(0, 11)], profile.year, profile.hour, profile.minute, profile.placeName),
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
        }
        Text(
            "Find the page for what you need. A gold dot marks the ones you have not opened yet; tap ☆ to keep a page in Favorites.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )

        var section = ""
        for (p in MORE_PAGES) {
            if (p.expert && !expert) continue
            if (p.section != section) {
                section = p.section
                SectionLabel(section)
                SECTION_LINES[section]?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = KshanaColors.TextSoft) }
            }
            PageRow(p.title, p.detail, p.route, open, fresh = p.section in NEED_SECTIONS && com.kshana.app.data.Discover.isNew(p.route))
        }
    }
}

private val MONTHS = listOf("Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec")

/** A page row: tap the words to open it, tap the star to keep it in Favorites. */
@Composable
internal fun PageRow(title: String, detail: String, route: String, onOpen: (String) -> Unit, fresh: Boolean = false) {
    val context = LocalContext.current
    val starred = Favorites.isPinned(route)
    val pop by animateFloatAsState(if (starred) 1.15f else 1f, spring(dampingRatio = 0.35f, stiffness = 400f), label = "star")
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(
        Modifier.fillMaxWidth().heightIn(min = 52.dp).padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f).clickable { onOpen(route) }.padding(vertical = 4.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(title, fontFamily = FontFamily.Serif, fontSize = 18.sp, color = KshanaColors.Text)
                if (fresh) {
                    Box(Modifier.padding(start = 8.dp).size(7.dp).background(KshanaColors.Gold, androidx.compose.foundation.shape.CircleShape))
                    Text("  NEW TO YOU", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                }
            }
            Text(detail, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
        Box(
            Modifier.size(44.dp).clickable { Favorites.toggle(context, route) }
                .semantics { contentDescription = if (starred) "Remove $title from favorites" else "Add $title to favorites" },
            contentAlignment = Alignment.Center,
        ) {
            Text(
                if (starred) "★" else "☆", fontSize = 22.sp,
                color = if (starred) KshanaColors.Gold else KshanaColors.Muted,
                modifier = Modifier.scale(pop),
            )
        }
        Text("›", fontSize = 22.sp, color = KshanaColors.Gold, modifier = Modifier.clickable { onOpen(route) }.padding(start = 2.dp))
    }
}
