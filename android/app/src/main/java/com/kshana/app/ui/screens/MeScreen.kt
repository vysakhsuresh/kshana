package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowRight
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Uncertainty
import com.kshana.app.engine.EngineProvider
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.KarakaRecord

/**
 * The fifth tab: who you are to the app, how it should speak, and every door that is
 * not a reading in itself (family, matching, naming, glossary, narration, the instrument).
 */
@Composable
fun MeScreen(
    profile: BirthProfile?,
    settings: EngineSettings,
    simple: Boolean,
    onSimpleChanged: (Boolean) -> Unit,
    onEditBirth: () -> Unit,
    onFamily: () -> Unit,
    onCompatibility: () -> Unit,
    onNaming: () -> Unit,
    onGlossary: () -> Unit,
    onChart: () -> Unit,
    onVedic: () -> Unit,
    onTrust: () -> Unit,
    onRectify: () -> Unit,
    onSettings: () -> Unit,
    onNarrate: () -> Unit,
) {
    val context = LocalContext.current
    var atmakaraka by remember { mutableStateOf<KarakaRecord?>(null) }

    LaunchedEffect(profile, settings) {
        atmakaraka = null
        if (profile == null) return@LaunchedEffect
        try {
            val engine = EngineProvider.get(context)
            val list = withContext(Dispatchers.Default) {
                engine.karakas(profile.birthInstant(), profile.latitude, profile.longitude, settings.ayanamsa)
            }
            atmakaraka = list.firstOrNull { it.name.equals("Atmakaraka", ignoreCase = true) }
                ?: list.minByOrNull { it.rank.toInt() }
        } catch (e: Exception) {
            // The line is optional; the screen stands without it.
            atmakaraka = null
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("ME", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)

        if (profile == null) {
            Text("No one yet", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
            Text(
                "Enter the date, clock time and place you were born and the sky is drawn for that moment. Everything stays on this phone.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
            )
            Spacer(Modifier.height(4.dp))
            GoldButton("ENTER YOUR BIRTH", onClick = onEditBirth, modifier = Modifier.fillMaxWidth())
        } else {
            Text(profile.name, style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                Text(
                    profile.birthLocal().format(DATE_TIME),
                    fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky,
                )
                Text(profile.placeName, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            }
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                Text("birth time", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                Chip(Uncertainty.label(profile.uncertaintyLevel).uppercase(), selected = false, onClick = onRectify)
            }
            val ak = atmakaraka
            if (ak != null) {
                Text("Atmakaraka: ${ak.graha}", style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text)
                if (ak.meaning.isNotBlank()) {
                    Text(ak.meaning, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                }
            }
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                QuietButton("EDIT", onClick = onEditBirth)
                QuietButton("FIX BIRTH TIME FROM LIFE EVENTS", onClick = onRectify, modifier = Modifier.weight(1f))
            }
        }

        SectionLabel("How the app speaks")
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Chip("PLAIN WORDS", selected = simple, onClick = { if (!simple) onSimpleChanged(true) })
            Chip("ASTROLOGER", selected = !simple, onClick = { if (simple) onSimpleChanged(false) })
        }
        Text(
            if (simple) "Readings in everyday words, with the reasons one tap away."
            else "The full instrument: cusps, sub lords, dashas and rule ids in every reading.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )

        SectionLabel("People and words")
        if (profile != null) {
            Door("Family", "People you've added: partner, children, parents, friends.", onFamily)
            Door("Compatibility", "Match two charts: the eight kutas, Mangal dosha and Western synastry.", onCompatibility)
            Door("Baby name syllable", "The traditional first sound of a name, from the Moon's star at birth.", onNaming)
        }
        Door("Glossary", "Every word the app uses, explained.", onGlossary)
        if (profile != null) {
            Door("Narration", "An optional online story of your reading, only if you allow it.", onNarrate)
        }

        SectionLabel("Chart details")
        Door("Western chart", "The tropical wheel: planets, houses and aspects.", onChart)
        Door("Vedic kundli", "The sidereal chart: rasi, navamsa, nakshatras and dashas.", onVedic)
        Door("Trust & verification", "What the app does not know, the self-test and the error budget.", onTrust)
        Door("Settings", "Ayanamsa, house system and the topocentric Moon.", onSettings)
        HorizontalDivider(color = KshanaColors.LineSoft)

        Spacer(Modifier.height(4.dp))
        Text("Everything is computed on this phone. Nothing leaves it unless you ask.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    }
}

/** A row with a serif title, a one-line description and a trailing chevron. */
@Composable
private fun Door(title: String, description: String, onClick: () -> Unit) {
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(
        Modifier.fillMaxWidth().heightIn(min = 56.dp).clickable(onClick = onClick).padding(vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text)
            Text(description, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        }
        Icon(Icons.Filled.KeyboardArrowRight, contentDescription = null, tint = KshanaColors.Gold, modifier = Modifier.padding(start = 12.dp))
    }
}
