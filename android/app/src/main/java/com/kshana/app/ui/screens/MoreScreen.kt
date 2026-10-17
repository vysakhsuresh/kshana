package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.ui.theme.KshanaColors

/** Everything beyond the loop: your birth details, settings, and the expert screens. */
@Composable
fun MoreScreen(profile: BirthProfile?, onOpen: (String) -> Unit) {
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("MORE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(profile?.name?.takeIf { it.isNotBlank() } ?: "Your chart", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        if (profile != null) {
            Text(
                "%d %s %d · %02d:%02d · %s".format(profile.day, MONTHS[(profile.month - 1).coerceIn(0, 11)], profile.year, profile.hour, profile.minute, profile.placeName),
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
        }

        SectionLabel("You")
        Entry("Birth details", "date, time, place and how sure you are", "birth", onOpen)
        Entry("Find the true minute", "rectify the birth time from things that happened", "rectify", onOpen)
        Entry("Settings", "ayanamsa, house system, where you are now", "settings", onOpen)
        Entry("Why trust this", "the engine, its data and its checks", "trust", onOpen)

        SectionLabel("Your life")
        Entry("What to do", "the remedies for what weighs on you now", "parihara", onOpen)
        Entry("What you are waiting for", "every date that matters, counting down", "wall", onOpen)
        Entry("The month's letter", "one page for the thirty days ahead", "letter", onOpen)
        Entry("Look back at a day", "any day of your life, read as it was", "hindsight", onOpen)
        Entry("Did it happen?", "grade what the app said; the record it keeps", "record", onOpen)
        Entry("The book of your life", "the years behind and ahead", "book", onOpen)

        SectionLabel("Go deeper")
        Entry("The sky now", "live wheel, ascendant, panchanga", "now", onOpen)
        Entry("Your birth chart", "Western wheel and aspects", "chart", onOpen)
        Entry("Vedic chart", "rasi, vargas, Vimshottari, Ashtakavarga", "vedic", onOpen)
        Entry("Life areas, in full", "KP promise and Parashari assessment with every rule", "areas", onOpen)
        Entry("Reading", "the strongest area now with its rule trace", "reading", onOpen)
        Entry("Exact moments", "ingresses, stations, lunations", "moments", onOpen)
        Entry("Muhurta", "the best windows for a purpose", "muhurta", onOpen)
        Entry("Prashna, horary", "the KP horary chart with a number, for the expert", "prashna_expert", onOpen)
    }
}

private val MONTHS = listOf("Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec")

@Composable
private fun Entry(title: String, detail: String, route: String, onOpen: (String) -> Unit) {
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(
        Modifier.fillMaxWidth().heightIn(min = 52.dp).clickable { onOpen(route) }.padding(vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(title, fontFamily = FontFamily.Serif, fontSize = 18.sp, color = KshanaColors.Text)
            Text(detail, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
        Text("›", fontSize = 22.sp, color = KshanaColors.Gold)
    }
}
