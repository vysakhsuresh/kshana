package com.kshana.app.ui.screens

import androidx.compose.foundation.background
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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
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
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.AnswerRecord
import uniffi.kshana_ffi.QuestionRecord
import uniffi.kshana_ffi.questions

/**
 * Ask: pick a life and a question, get a plain answer deduced from the birth chart, the
 * running periods, the sky ahead and the horary chart of the moment. No typing, no AI.
 */
@Composable
fun AskScreen(
    profile: BirthProfile,
    place: Place,
    settings: EngineSettings,
    initialLife: String? = null,
    onBack: (() -> Unit)? = null,
    onRemedy: (String) -> Unit = {},
) {
    val context = LocalContext.current
    val qs: List<QuestionRecord> = remember { questions() }
    var life by remember { mutableStateOf(initialLife ?: "Love") }
    var question by remember { mutableStateOf(qs.firstOrNull()?.key ?: "will_it_happen") }
    var answer by remember { mutableStateOf<AnswerRecord?>(null) }
    var asking by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var askedKey by remember { mutableStateOf<String?>(null) }
    var showWhy by remember { mutableStateOf(false) }

    LaunchedEffect(askedKey) {
        val key = askedKey ?: return@LaunchedEffect
        asking = true
        try {
            val engine = EngineProvider.get(context)
            val at = Instants.now()
            answer = withContext(Dispatchers.Default) {
                engine.ask(
                    profile.birthInstant(), profile.latitude, profile.longitude,
                    place.latitude, place.longitude,
                    settings.houseSystem, settings.ayanamsa, life, question, at,
                )
            }
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
        asking = false
        showWhy = false
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("ASK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            if (onBack != null) Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("Ask the sky", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text("Pick what it is about and what you want to know. The answer comes from your birth chart, the running periods, the months ahead and the sky at this very second.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)

        SectionLabel("About")
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (l in LIFE_ORDER) Chip("${lifeGlyph(l)} ${l.uppercase()}", selected = life == l, onClick = { life = l; answer = null })
        }
        SectionLabel("Question")
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            for (row in qs.chunked(2)) {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    for (q in row) {
                        OptionBox(q.words, selected = question == q.key, onClick = { question = q.key; answer = null }, modifier = Modifier.weight(1f))
                    }
                    if (row.size == 1) Spacer(Modifier.weight(1f))
                }
            }
        }
        GoldButton(
            if (asking) "READING THE SKY…" else "ASK · ${life.uppercase()}",
            enabled = !asking,
            onClick = { askedKey = "$life|$question|${System.currentTimeMillis()}" },
            modifier = Modifier.fillMaxWidth(),
        )
        ErrorLine(error)

        val a = answer
        if (a != null) {
            Spacer(Modifier.height(4.dp))
            Column(
                Modifier.fillMaxWidth().clip(RoundedCornerShape(18.dp)).background(KshanaColors.Surface).padding(18.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                    ScoreRing(a.score, size = 84.dp, stroke = 7.dp, label = "${a.confidence.toInt()}% sure")
                    Column(Modifier.weight(1f)) {
                        Text("${lifeGlyph(a.life)} ${a.life} · ${qs.firstOrNull { it.key == a.question }?.words ?: a.question}", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                        Text(a.verdict, style = MaterialTheme.typography.displayMedium, color = scoreColor(a.score))
                    }
                }
                Text(a.headline, fontFamily = FontFamily.Serif, fontSize = 20.sp, lineHeight = 28.sp, color = KshanaColors.Text)
                val ws = a.whenStart
                val we = a.whenEnd
                if (ws != null && we != null) {
                    HorizontalDivider(color = KshanaColors.LineSoft)
                    Text(a.whenWords.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                    Text(
                        if (sameDay(ws, we)) "${localText(ws, WEEKDAY_DAY)} · ${hm(ws)} – ${hm(we)}" else "${localText(ws, WEEKDAY_DAY)} – ${localText(we, WEEKDAY_DAY)}",
                        fontFamily = FontFamily.Monospace, fontSize = 16.sp, color = KshanaColors.Sky,
                    )
                }
                val os = a.firstOpeningStart
                val oe = a.firstOpeningEnd
                if (os != null && oe != null) {
                    Text("First opening by the ruling planets", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                    Text("${localText(os, WEEKDAY_DAY)} ${hm(os)} – ${if (sameDay(os, oe)) "" else localText(oe, WEEKDAY_DAY) + " "}${hm(oe)}", fontFamily = FontFamily.Monospace, fontSize = 13.sp, color = KshanaColors.Sky)
                }
                Row(horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                    Stat("now", a.now)
                    Stat("6 months", a.aheadMean)
                    Stat("best ahead", a.aheadBest)
                }
            }

            Text(
                if (showWhy) "HIDE THE REASONS −" else "WHY? +",
                style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                modifier = Modifier.clickable { showWhy = !showWhy }.padding(vertical = 4.dp),
            )
            if (showWhy) {
                for ((i, r) in a.reasons.withIndex()) {
                    HorizontalDivider(color = KshanaColors.LineSoft)
                    Row(Modifier.padding(vertical = 6.dp), verticalAlignment = Alignment.Top) {
                        Box(Modifier.padding(top = 6.dp).size(8.dp).clip(CircleShape).background(KshanaColors.Sky))
                        Spacer(Modifier.width(10.dp))
                        Column(Modifier.weight(1f)) {
                            Text(r.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                            a.ruleIds.getOrNull(i)?.let { Text("rule $it", fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted) }
                        }
                    }
                }
                if (a.shifts.isNotEmpty()) {
                    SectionLabel("What changes ahead")
                    for (s in a.shifts) {
                        Row(Modifier.fillMaxWidth().padding(vertical = 2.dp)) {
                            Text(localText(s.at, WEEKDAY_DAY), fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky, modifier = Modifier.width(92.dp))
                            Text(s.what, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
                        }
                    }
                }
            }
            if (a.score < 56) {
                QuietButton("REMEDY FOR ${a.life.uppercase()}", onClick = { onRemedy(a.life) }, modifier = Modifier.fillMaxWidth())
            }
            Text("Asked ${localText(Instants.now(), DATE_TIME_S)} at ${place.name}. Computed on this phone; nothing leaves it.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
    }
}

@Composable
private fun Stat(label: String, v: Double) {
    Column {
        Text(label.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        Text(v.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 18.sp, color = scoreColor(v))
    }
}

private fun sameDay(a: uniffi.kshana_ffi.ExactInstant, b: uniffi.kshana_ffi.ExactInstant): Boolean =
    Instants.toLocal(a).toLocalDate() == Instants.toLocal(b).toLocalDate()
