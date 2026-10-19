package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
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
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
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
import com.kshana.app.data.FamilyStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.CompatibilityRecord
import uniffi.kshana_ffi.MangalRecord
import com.kshana.app.ui.sky.KshanaFonts

/** Two people, matched: the eight kutas, the doshas, Mangal, Western synastry, and the sentences. */
@Composable
fun CompatibilityScreen(settings: EngineSettings, onFamily: () -> Unit, onBack: () -> Unit) {
    val context = LocalContext.current
    val people = remember { FamilyStore.everyone(context) }
    var indexA by remember { mutableIntStateOf(0) }
    var indexB by remember { mutableIntStateOf(if (people.size > 1) 1 else 0) }
    var request by remember { mutableStateOf<Pair<BirthProfile, BirthProfile>?>(null) }
    var result by remember { mutableStateOf<CompatibilityRecord?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }

    LaunchedEffect(request, settings) {
        val r = request ?: return@LaunchedEffect
        busy = true
        try {
            val engine = EngineProvider.get(context)
            val (a, b) = r
            result = withContext(Dispatchers.Default) {
                engine.compatibility(a.birthInstant(), a.latitude, a.longitude, b.birthInstant(), b.latitude, b.longitude, settings.ayanamsa)
            }
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
        busy = false
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("COMPATIBILITY", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("Two skies, matched", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)

        if (people.size < 2) {
            Text(
                "Matching needs two births. Add a partner or anyone else under Family and both will appear here to choose from.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
            )
            QuietButton("ADD A PERSON", onClick = onFamily)
            return@Column
        }

        SectionLabel("Person A")
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((i, p) in people.withIndex()) Chip(p.name.uppercase(), selected = i == indexA, onClick = { indexA = i })
        }
        SectionLabel("Person B")
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((i, p) in people.withIndex()) Chip(p.name.uppercase(), selected = i == indexB, onClick = { indexB = i })
        }
        if (indexA == indexB) {
            Text("Choose two different people.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
        Spacer(Modifier.height(4.dp))
        GoldButton(
            if (busy) "MATCHING…" else "MATCH",
            enabled = indexA != indexB && !busy,
            onClick = { request = people[indexA] to people[indexB] },
            modifier = Modifier.fillMaxWidth(),
        )
        ErrorLine(error)

        val c = result
        val pair = request
        if (c != null && pair != null) {
            val maxTotal = c.kutas.sumOf { it.max }.let { if (it > 0.0) it else 36.0 }
            SectionLabel("${pair.first.name} · ${pair.second.name}")
            Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Text("${num(c.total)} / ${num(maxTotal)}", style = MaterialTheme.typography.headlineLarge, color = KshanaColors.Sky)
                Text(c.verdict, style = MaterialTheme.typography.displayMedium, color = KshanaColors.Gold, modifier = Modifier.padding(bottom = 4.dp))
            }

            SectionLabel("Ashtakoota")
            for (k in c.kutas) {
                Column(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                        Text(k.name, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text, modifier = Modifier.weight(1f))
                        Text("${num(k.points)}/${num(k.max)}", fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky)
                    }
                    Bar(fraction = if (k.max > 0.0) (k.points / k.max).toFloat() else 0f)
                    if (k.detail.isNotBlank()) {
                        Text(k.detail, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(top = 2.dp))
                    }
                }
            }
            if (c.nadiDosha) Text("Nadi dosha: both charts share the same nadi.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Gold)
            if (c.bhakootDosha) Text("Bhakoot dosha: the Moon signs stand in a difficult relation.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Gold)

            SectionLabel("Mangal dosha")
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                MangalCard(pair.first.name, c.mangalA, Modifier.weight(1f))
                MangalCard(pair.second.name, c.mangalB, Modifier.weight(1f))
            }

            SectionLabel("Western synastry")
            val top = remember(c) { c.synastry.sortedByDescending { it.weight }.take(8) }
            if (top.isEmpty()) {
                Text("No cross-aspects within orb.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            }
            for (s in top) {
                val toneWord = when { s.tone > 0 -> "harmonious"; s.tone < 0 -> "tense"; else -> "neutral" }
                val toneColor = when { s.tone > 0 -> KshanaColors.Sky; s.tone < 0 -> KshanaColors.Gold; else -> KshanaColors.Muted }
                HorizontalDivider(color = KshanaColors.LineSoft)
                Row(Modifier.fillMaxWidth().heightIn(min = 44.dp).padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text("${s.a} ${s.aspect} ${s.b}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
                    Text("%.2f°".format(s.orb), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky, modifier = Modifier.padding(horizontal = 10.dp))
                    Text(toneWord, style = MaterialTheme.typography.labelSmall, color = toneColor)
                }
            }

            SectionLabel("In words")
            if (c.sentences.isEmpty()) {
                Text("The engine wrote no sentences for this pair.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            }
            for ((i, s) in c.sentences.withIndex()) SentenceRow(i, s.text, s.why, s.confidence, s.rules)
        }
    }
}

private fun num(x: Double): String = if (x % 1.0 == 0.0) "%.0f".format(x) else "%.1f".format(x)

@Composable
private fun Bar(fraction: Float) {
    val shape = RoundedCornerShape(3.dp)
    Box(Modifier.fillMaxWidth().height(6.dp).clip(shape).background(KshanaColors.LineSoft)) {
        Box(Modifier.fillMaxWidth(fraction.coerceIn(0f, 1f)).height(6.dp).background(KshanaColors.Gold))
    }
}

@Composable
private fun MangalCard(name: String, m: MangalRecord, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(8.dp)
    val headline = when {
        !m.present -> "No Mangal dosha"
        !m.effective -> "Mangal dosha, cancelled by ${m.cancellations.joinToString(", ").ifBlank { "the chart" }}"
        else -> "Mangal dosha, uncancelled"
    }
    Column(modifier.clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(name, style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text)
        Text(headline, style = MaterialTheme.typography.bodyMedium, color = if (m.effective) KshanaColors.Gold else KshanaColors.TextSoft)
        Text(
            "Mars in house ${m.fromLagna} from lagna · ${m.fromMoon} from Moon · ${m.fromVenus} from Venus",
            fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Sky,
        )
    }
}

@Composable
private fun SentenceRow(key: Int, text: String, why: List<String>, confidence: Double, rules: List<String>) {
    var open by remember(key) { mutableStateOf(false) }
    HorizontalDivider(color = KshanaColors.LineSoft)
    Column(Modifier.fillMaxWidth().heightIn(min = 44.dp).clickable { open = !open }.padding(vertical = 8.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(sentence(text), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
            Text(if (open) "why −" else "why +", fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Gold, modifier = Modifier.padding(start = 12.dp))
        }
        if (open) {
            for (w in why) {
                Text("· $w", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Sky, modifier = Modifier.padding(top = 3.dp))
            }
            Text(
                "confidence ${"%.2f".format(confidence)}${if (rules.isNotEmpty()) " · rules ${rules.joinToString(" ")}" else ""}",
                fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted, modifier = Modifier.padding(top = 4.dp),
            )
        }
    }
}
