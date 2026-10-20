package com.kshana.app.ui.screens

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.clickable
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

/** Every page under More, in the order shown. The Favorites tab reads the same list. */
internal val MORE_PAGES = listOf(
    MorePage("You", "Birth details", "date, time, place and how sure you are", "birth"),
    MorePage("You", "Find your exact birth time", "work it out from things that happened in your life", "rectify"),
    MorePage("You", "Settings", "where you are, language, the sunrise reading", "settings"),

    MorePage("Help", "Guide", "how to use Kshana, screen by screen", "guide"),
    MorePage("Help", "How Kshana works", "what it reads, how sure it is, your data, privacy", "how"),
    MorePage("Help", "About Kshana", "who we are, what the name means, the version", "about"),

    MorePage("Your life", "What to do", "the remedies for what weighs on you now", "parihara"),
    MorePage("Your life", "What you are waiting for", "every date that matters, counting down", "wall"),
    MorePage("Your life", "The month's letter", "one page for the thirty days ahead", "letter"),
    MorePage("Your life", "Check it against your past", "a few stretches of your last years: did they happen?", "pastcheck"),
    MorePage("Your life", "A date for all of us", "a wedding, a house-warming, a trip: the hours that suit everyone", "familydates"),
    MorePage("Your life", "Your journal", "write down what happened; see how often the app was right", "journal"),
    MorePage("Your life", "Your week's appointments", "your calendar, checked against your hours", "calendar"),
    MorePage("Your life", "Look back at a day", "any day of your life, read as it was", "hindsight"),
    MorePage("Your life", "Did it happen?", "say whether the app was right; it keeps score", "record"),
    MorePage("Your life", "The book of your life", "the years behind and ahead", "book"),

    MorePage("Chart pages", "Why trust this", "the engine, its data and its checks", "trust", true),
    MorePage("Chart pages", "The sky now", "live wheel, ascendant, panchanga", "now", true),
    MorePage("Chart pages", "Remedies, classic list", "every remedy for the running periods, with sources", "parihara_classic", true),
    MorePage("Chart pages", "Your birth chart", "Western wheel and aspects", "chart", true),
    MorePage("Chart pages", "Vedic chart", "rasi, vargas, Vimshottari, Ashtakavarga", "vedic", true),
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

/** Everything beyond the tabs: your birth details, help, your life's pages and the chart pages. */
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
        Text("MORE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(profile?.name?.takeIf { it.isNotBlank() } ?: "You", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        if (profile != null) {
            Text(
                "%d %s %d · %02d:%02d · %s".format(profile.day, MONTHS[(profile.month - 1).coerceIn(0, 11)], profile.year, profile.hour, profile.minute, profile.placeName),
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
        }
        Text(
            "Tap ☆ beside any page to keep it in Favorites.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )

        var section = ""
        for (p in MORE_PAGES) {
            if (p.expert && !expert) continue
            if (p.section != section) { section = p.section; SectionLabel(section) }
            PageRow(p.title, p.detail, p.route, open)
        }
    }
}

private val MONTHS = listOf("Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec")

/** A page row: tap the words to open it, tap the star to keep it in Favorites. */
@Composable
internal fun PageRow(title: String, detail: String, route: String, onOpen: (String) -> Unit) {
    val context = LocalContext.current
    val starred = Favorites.isPinned(route)
    val pop by animateFloatAsState(if (starred) 1.15f else 1f, spring(dampingRatio = 0.35f, stiffness = 400f), label = "star")
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(
        Modifier.fillMaxWidth().heightIn(min = 52.dp).padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f).clickable { onOpen(route) }.padding(vertical = 4.dp)) {
            Text(title, fontFamily = FontFamily.Serif, fontSize = 18.sp, color = KshanaColors.Text)
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
