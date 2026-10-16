package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.SectionRecord
import uniffi.kshana_ffi.SentenceRecord

private const val WHO = "Who you are"
private const val NOW = "Right now"
private const val NEXT = "The next 10 years"

/**
 * The plain-words reading as cards: who you are, right now, the eight life areas as
 * tiles that open in place, and the years ahead. The reading is computed once and kept.
 */
@Composable
fun LifeScreen(
    profile: BirthProfile,
    settings: EngineSettings,
    simple: Boolean,
    onLifeMap: () -> Unit,
    onAreas: () -> Unit,
    onParihara: () -> Unit,
    onChart: () -> Unit,
) {
    val context = LocalContext.current
    var sections by remember { mutableStateOf<List<SectionRecord>>(emptyList()) }
    var error by remember { mutableStateOf<String?>(null) }
    var loading by remember { mutableStateOf(true) }
    var expanded by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(profile, settings) {
        loading = true
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            sections = withContext(Dispatchers.Default) {
                engine.plainReading(birth, profile.latitude, profile.longitude, settings.ayanamsa, Instants.now())
            }
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
        loading = false
    }

    // Split once; the cards only re-read these when the reading itself changes.
    val who = remember(sections) { sections.firstOrNull { it.title == WHO } }
    val rightNow = remember(sections) { sections.firstOrNull { it.title == NOW } }
    val next = remember(sections) { sections.firstOrNull { it.title == NEXT } }
    val areas = remember(sections) { sections.filter { it.title != WHO && it.title != NOW && it.title != NEXT } }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("YOUR LIFE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(profile.name.trim().ifEmpty { "Your reading" }, style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        ErrorLine(error)

        if (loading && sections.isEmpty()) {
            Text("Reading your chart…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        }
        if (!loading && sections.isEmpty() && error == null) {
            Text("The engine returned no reading.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        }

        who?.let { ReadingCard(it, showWhy = !simple) }
        rightNow?.let { ReadingCard(it, showWhy = !simple) }

        if (areas.isNotEmpty()) {
            SectionLabel("Life areas")
            for (pair in areas.chunked(2)) {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    for (s in pair) {
                        AreaTile(s, selected = expanded == s.title, modifier = Modifier.weight(1f)) {
                            expanded = if (expanded == s.title) null else s.title
                        }
                    }
                    if (pair.size == 1) Spacer(Modifier.weight(1f))
                }
                val open = pair.firstOrNull { it.title == expanded }
                if (open != null) AreaDetail(open)
            }
        }

        next?.let { ReadingCard(it, showWhy = !simple) }

        if (sections.isNotEmpty()) {
            Spacer(Modifier.height(6.dp))
            GoldButton("SEE YOUR LIFE MAP", onClick = onLifeMap, modifier = Modifier.fillMaxWidth())
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                QuietButton("TIMING", onClick = onAreas, modifier = Modifier.weight(1f))
                QuietButton("REMEDIES", onClick = onParihara, modifier = Modifier.weight(1f))
                if (!simple) QuietButton("CHART", onClick = onChart, modifier = Modifier.weight(1f))
            }
        }
    }
}

/** A card: the section label, the summary as a serif title, the sentences as paragraphs. */
@Composable
private fun ReadingCard(section: SectionRecord, showWhy: Boolean) {
    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text(section.title.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        if (section.summary.isNotBlank()) {
            Text(
                section.summary,
                style = if (section.summary.length <= 36) MaterialTheme.typography.displayMedium else MaterialTheme.typography.titleLarge,
                color = KshanaColors.Text,
            )
        }
        for ((i, s) in section.sentences.withIndex()) {
            if (showWhy) SentenceBlock(s, key = "${section.title}:$i")
            else Text(s.text, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        }
    }
}

/** One of the eight areas: title and a promised / to be earned mark. */
@Composable
private fun AreaTile(section: SectionRecord, selected: Boolean, modifier: Modifier = Modifier, onClick: () -> Unit) {
    val shape = RoundedCornerShape(8.dp)
    val promised = section.summary.trim().equals("promised", ignoreCase = true)
    Column(
        modifier
            .heightIn(min = 84.dp)
            .clip(shape)
            .background(KshanaColors.Surface)
            .border(1.dp, if (selected) KshanaColors.Gold else KshanaColors.Line, shape)
            .clickable(onClick = onClick)
            .padding(14.dp),
        verticalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(section.title, fontFamily = FontFamily.Serif, fontSize = 18.sp, color = KshanaColors.Text)
        Spacer(Modifier.height(8.dp))
        val mark = if (promised) KshanaColors.Gold else KshanaColors.Muted
        Box(
            Modifier.border(1.dp, mark, RoundedCornerShape(11.dp)).padding(horizontal = 8.dp, vertical = 4.dp),
        ) {
            Text(section.summary.uppercase(), style = MaterialTheme.typography.labelSmall, color = mark)
        }
    }
}

/** The opened area, under its row of tiles. */
@Composable
private fun AreaDetail(section: SectionRecord) {
    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Gold.copy(alpha = 0.5f), shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("${section.title} · ${section.summary}", fontFamily = FontFamily.Serif, fontSize = 17.sp, color = KshanaColors.Text)
        if (section.sentences.isEmpty()) {
            Text("Nothing more to say for this area.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        }
        for ((i, s) in section.sentences.withIndex()) SentenceBlock(s, key = "${section.title}:$i")
    }
}

/** A sentence, a thin gold confidence bar, and a "why" toggle that reveals the reasons and rule ids. */
@Composable
private fun SentenceBlock(s: SentenceRecord, key: String) {
    var open by remember(key) { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth()) {
        Text(s.text, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        Row(Modifier.fillMaxWidth().padding(top = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.weight(1f).height(2.dp).background(KshanaColors.LineSoft)) {
                Box(Modifier.fillMaxWidth(s.confidence.coerceIn(0.0, 1.0).toFloat()).height(2.dp).background(KshanaColors.Gold))
            }
            Text(
                if (open) "WHY −" else "WHY +",
                style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                modifier = Modifier.clickable { open = !open }.padding(start = 12.dp, top = 8.dp, bottom = 8.dp),
            )
        }
        if (open) {
            if (s.why.isEmpty()) Text("no reasons recorded", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            for (w in s.why) Text("· $w", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky, modifier = Modifier.padding(top = 2.dp))
            Text(
                "confidence ${"%.2f".format(s.confidence)}${if (s.rules.isNotEmpty()) " · rules ${s.rules.joinToString(" ")}" else ""}",
                fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted, modifier = Modifier.padding(top = 4.dp),
            )
        }
    }
}
