package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.data.PredictionStore
import com.kshana.app.engine.Instants
import uniffi.kshana_ffi.calibrate
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import uniffi.kshana_ffi.HorizonRecord
import uniffi.kshana_ffi.HourRecord
import uniffi.kshana_ffi.MomentRecord
import uniffi.kshana_ffi.StoryRecord
import java.time.Instant

private val RANGES = listOf("TODAY", "WEEK", "MONTH", "YEAR")

/** One life: now, then today, this week, this month, this year, in plain words. */
@Composable
fun LifeScreen(
    profile: BirthProfile,
    place: Place,
    settings: EngineSettings,
    life: String,
    onBack: () -> Unit,
    onAsk: () -> Unit,
    onRemedy: () -> Unit,
) {
    val context = LocalContext.current
    var moment by remember { mutableStateOf<MomentRecord?>(null) }
    var horizons by remember { mutableStateOf<Map<String, HorizonRecord>>(emptyMap()) }
    var range by remember { mutableStateOf("TODAY") }
    var error by remember { mutableStateOf<String?>(null) }
    var showWhy by remember { mutableStateOf(false) }
    val idx = LIFE_ORDER.indexOf(life).coerceAtLeast(0)

    LaunchedEffect(profile, place, settings) {
        try {
            moment = loadMoment(context, profile, place, settings)
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }
    LaunchedEffect(profile, place, settings, range) {
        if (range == "TODAY" || horizons.containsKey(range)) return@LaunchedEffect
        try {
            val now = Instants.now()
            val h = when (range) {
                "WEEK" -> loadHorizon(context, profile, place, settings, now, 1.0, 8)
                "MONTH" -> loadHorizon(context, profile, place, settings, now, 1.0, 31)
                else -> loadHorizon(context, profile, place, settings, now, 7.0, 53)
            }
            horizons = horizons + (range to h)
            // Remember the named windows of the week, so they can be judged afterwards.
            if (range == "WEEK") {
                for (s in h.stories) {
                    val bs = s.bestStart
                    val be = s.bestEnd
                    if (bs != null && be != null) PredictionStore.remember(context, s.life, bs.toJava().toEpochMilli(), be.toJava().toEpochMilli() + 86_400_000L, s.bestScore)
                    val ls = s.lowStart
                    val le = s.lowEnd
                    if (ls != null && le != null) PredictionStore.remember(context, s.life, ls.toJava().toEpochMilli(), le.toJava().toEpochMilli() + 86_400_000L, s.lowScore)
                }
            }
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    val m = moment
    val now = m?.lives?.getOrNull(idx)
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("${lifeGlyph(life)}  ${life.uppercase()}", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text(lifeQuestion(life), style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        ErrorLine(error)

        if (now != null) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                ScoreRing(now.score, size = 84.dp, stroke = 7.dp, label = bandWord(now.band))
                Column(Modifier.weight(1f)) {
                    Text(now.headline, fontFamily = FontFamily.Serif, fontSize = 20.sp, lineHeight = 27.sp, color = KshanaColors.Text)
                    Text(now.because.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                }
            }
            Text(
                if (showWhy) "LESS −" else "WHAT LIES BEHIND IT +",
                style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                modifier = Modifier.clickable { showWhy = !showWhy }.padding(vertical = 4.dp),
            )
            if (showWhy) {
                for (c in now.components) {
                    HorizontalDivider(color = KshanaColors.LineSoft)
                    Row(Modifier.padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
                        Box(Modifier.size(8.dp).clip(CircleShape).background(scoreColor(c.score * 100)))
                        Spacer(Modifier.width(10.dp))
                        Column(Modifier.weight(1f)) {
                            Text(c.note.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                            Text("${componentName(c.key)} · ${(c.score * 100).toInt()}", fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted)
                        }
                    }
                }
            }
        } else if (error == null) {
            Working("reading your day…")
        }

        if (now != null) ExplainPanel(profile, place, settings, life, onRemedy = onRemedy)
        RecordPanel(life)

        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (r in RANGES) Chip(r, selected = range == r, onClick = { range = r })
        }

        when (range) {
            "TODAY" -> if (m != null) TodayPanel(m.hours, idx, life)
            else -> {
                val h = horizons[range]
                if (h == null) {
                    Working("looking ahead…")
                } else {
                    HorizonPanel(h, idx, range)
                }
            }
        }

        Spacer(Modifier.height(4.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            GoldButton("ASK", onClick = onAsk, modifier = Modifier.weight(1f))
            QuietButton("WHAT TO DO", onClick = onRemedy, modifier = Modifier.weight(1f))
        }
    }
}

/**
 * "Did this happen?": the windows the app named for this life that have passed, each
 * answered with one tap, and what the record says so far (`calibrate()` in the engine).
 */
@Composable
internal fun RecordPanel(life: String) {
    val context = LocalContext.current
    var version by remember { mutableStateOf(0) }
    val pending = remember(version) { PredictionStore.pending(context, life).takeLast(4) }
    val calibration = remember(version) { calibrate(PredictionStore.records(context)) }
    val mine = calibration.lives.firstOrNull { it.life == life }
    if (pending.isEmpty() && (mine == null || mine.n == 0u)) return
    Column(
        Modifier.fillMaxWidth().glass(14.dp).padding(14.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text("YOUR RECORD · DID IT HAPPEN?", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        for (p in pending) {
            val s = java.time.Instant.ofEpochMilli(p.start).atZone(java.time.ZoneId.systemDefault())
            val e = java.time.Instant.ofEpochMilli(p.end - 1).atZone(java.time.ZoneId.systemDefault())
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text("${s.format(WEEKDAY_DAY)} – ${e.format(WEEKDAY_DAY)}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky)
                    Text("The app read $life at ${p.score.toInt()} (${bandWord(scoreBandWord(p.score)).lowercase()}). Did it go that way?", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                }
                Text("YES", style = MaterialTheme.typography.labelMedium, color = KshanaColors.Leaf, modifier = Modifier.clickable { PredictionStore.answer(context, life, p.start, true); version++ }.padding(8.dp))
                Text("NO", style = MaterialTheme.typography.labelMedium, color = KshanaColors.Ember, modifier = Modifier.clickable { PredictionStore.answer(context, life, p.start, false); version++ }.padding(8.dp))
            }
        }
        if (mine != null && mine.n > 0u) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            Text(mine.words, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
            if (mine.offset != 0.0) Text("By your record: ${signedPoints(mine.offset)} on $life. Shown, never hidden in the score.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Gold)
        }
    }
}

private fun scoreBandWord(score: Double): String = when {
    score >= 70 -> "bright"; score >= 56 -> "good"; score >= 44 -> "steady"; score >= 32 -> "slow"; else -> "hold"
}

private fun componentName(key: String): String = when (key) {
    "natal" -> "What you were born with"
    "bhava" -> "Your own timing for this"
    "dasha" -> "This stretch of your life"
    "gochara" -> "This season"
    "tara" -> "Today's luck"
    "hora" -> "This hour"
    "kala" -> "The time of day"
    else -> key
}

@Composable
private fun TodayPanel(hours: List<HourRecord>, idx: Int, life: String) {
    if (hours.isEmpty()) {
        Text("No sunrise and sunset here today, so the hours cannot be drawn.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        return
    }
    val now = Instant.now()
    val fits = hours.map { it.fit.getOrNull(idx) ?: 50.0 }
    SectionLabel("The 24 hours · sunrise to sunrise")
    Canvas(Modifier.fillMaxWidth().height(96.dp)) {
        val n = hours.size
        val gap = 3.dp.toPx()
        val w = (size.width - gap * (n - 1)) / n
        hours.forEachIndexed { i, h ->
            val v = (fits[i] / 100.0).toFloat().coerceIn(0.04f, 1f)
            val x = i * (w + gap)
            val top = size.height * (1 - v)
            val current = !now.isBefore(h.start.toJava()) && now.isBefore(h.end.toJava())
            val past = !now.isBefore(h.end.toJava())
            val c = when {
                h.caution -> KshanaColors.Ember
                else -> scoreColor(fits[i])
            }
            drawRoundRect(
                c.copy(alpha = if (past) 0.35f else 1f),
                topLeft = Offset(x, top),
                size = Size(w, size.height - top),
                cornerRadius = androidx.compose.ui.geometry.CornerRadius(3.dp.toPx()),
            )
            if (current) {
                drawRoundRect(
                    KshanaColors.Gold,
                    topLeft = Offset(x - 1.5f, 0f),
                    size = Size(w + 3f, size.height),
                    cornerRadius = androidx.compose.ui.geometry.CornerRadius(4.dp.toPx()),
                    style = Stroke(2f),
                )
            }
        }
    }
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(hm(hours.first().start), fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted)
        Text("sunset", fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted)
        Text(hm(hours.last().end), fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted)
    }
    val upcoming = hours.withIndex().filter { !it.value.caution && it.value.end.toJava().isAfter(now) }
    val best = upcoming.filter { fits[it.index] >= 45.0 }.sortedByDescending { fits[it.index] }.take(3).sortedBy { it.index }
    SectionLabel("Best hours left today for ${life.lowercase()}")
    if (best.isEmpty()) {
        Text("The day's good hours have passed; tomorrow's sunrise starts a new set.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
    }
    for (b in best) {
        val h = b.value
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("${hm(h.start)} – ${hm(h.end)}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky, modifier = Modifier.width(110.dp))
            Text(hourNature(h.lord).replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text, modifier = Modifier.weight(1f))
            Text(fits[b.index].toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 14.sp, color = scoreColor(fits[b.index]))
        }
    }
    val cautions = hours.filter { it.caution && it.end.toJava().isAfter(now) }
    if (cautions.isNotEmpty()) {
        Text(
            "Hours to pause: " + cautions.joinToString(", ") { "${hm(it.start)}–${hm(it.end)}" },
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Ember,
        )
    }
}

@Composable
private fun HorizonPanel(h: HorizonRecord, idx: Int, range: String) {
    val values = h.samples.map { it.scores.getOrNull(idx) ?: 50.0 }
    val story = h.stories.getOrNull(idx)
    SectionLabel(
        when (range) {
            "WEEK" -> "The next seven days"
            "MONTH" -> "The next thirty days"
            else -> "The next twelve months"
        },
    )
    Sparkline(values)
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(h.samples.firstOrNull()?.let { dayMon(it.at) } ?: "", fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted)
        Text(h.samples.lastOrNull()?.let { dayMon(it.at) } ?: "", fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted)
    }
    if (story != null) StoryBlock(story, range)
}

@Composable
private fun StoryBlock(s: StoryRecord, range: String) {
    Text(s.trendWords, fontFamily = FontFamily.Serif, fontSize = 20.sp, lineHeight = 27.sp, color = KshanaColors.Text)
    Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Text("average ${s.mean.toInt()}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = scoreColor(s.mean))
        Text("· ${s.trend}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Muted)
    }
    val bs = s.bestStart
    val be = s.bestEnd
    if (bs != null && be != null) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(8.dp).clip(CircleShape).background(KshanaColors.Gold))
            Spacer(Modifier.width(10.dp))
            Text("Best: ${span(bs, be, range)}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text, modifier = Modifier.weight(1f))
            Text(s.bestScore.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 14.sp, color = KshanaColors.Gold)
        }
    }
    val ls = s.lowStart
    val le = s.lowEnd
    if (ls != null && le != null) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(8.dp).clip(CircleShape).background(KshanaColors.Ember))
            Spacer(Modifier.width(10.dp))
            Text("Go easy: ${span(ls, le, range)}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text, modifier = Modifier.weight(1f))
            Text(s.lowScore.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 14.sp, color = KshanaColors.Ember)
        }
    }
    if (s.shifts.isNotEmpty()) {
        SectionLabel("What changes")
        for (sh in s.shifts) {
            Row(Modifier.fillMaxWidth().padding(vertical = 2.dp)) {
                Text(localText(sh.at, WEEKDAY_DAY), fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky, modifier = Modifier.width(92.dp))
                Text(sh.what, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
            }
        }
    }
}

private fun span(a: uniffi.kshana_ffi.ExactInstant, b: uniffi.kshana_ffi.ExactInstant, range: String): String {
    val fa = if (range == "WEEK") localText(a, WEEKDAY_DAY) else dayMon(a)
    val fb = if (range == "WEEK") localText(b, WEEKDAY_DAY) else dayMon(b)
    return if (fa == fb) fa else "$fa – $fb"
}

/** A line through the samples with the band colour, a gold dot at the peak and an ember dot at the trough. */
@Composable
private fun Sparkline(values: List<Double>) {
    if (values.size < 2) return
    // Zoom the axis to the data (at least a 30-point span) so the shape is readable.
    val lo = ((values.min() - 8.0).coerceAtLeast(0.0))
    val hi = ((values.max() + 8.0).coerceAtMost(100.0))
    val span = (hi - lo).coerceAtLeast(30.0)
    val bottom = if (hi - lo < 30.0) ((lo + hi) / 2 - 15.0).coerceIn(0.0, 70.0) else lo
    Canvas(Modifier.fillMaxWidth().height(110.dp)) {
        val n = values.size
        val stepX = size.width / (n - 1)
        fun y(v: Double) = (size.height * (1 - ((v - bottom) / span).coerceIn(0.0, 1.0))).toFloat()
        // band guides
        for (g in listOf(32.0, 44.0, 56.0, 70.0)) {
            if (g > bottom && g < bottom + span) {
                drawLine(KshanaColors.LineSoft, Offset(0f, y(g)), Offset(size.width, y(g)), 1f)
            }
        }
        val path = Path()
        values.forEachIndexed { i, v ->
            val p = Offset(i * stepX, y(v))
            if (i == 0) path.moveTo(p.x, p.y) else path.lineTo(p.x, p.y)
        }
        drawPath(path, KshanaColors.Sky, style = Stroke(2.5f, cap = StrokeCap.Round))
        val maxI = values.indices.maxByOrNull { values[it] } ?: 0
        val minI = values.indices.minByOrNull { values[it] } ?: 0
        drawCircle(KshanaColors.Gold, 5f, Offset(maxI * stepX, y(values[maxI])))
        drawCircle(KshanaColors.Ember, 5f, Offset(minI * stepX, y(values[minI])))
        drawCircle(KshanaColors.Text, 4f, Offset(0f, y(values[0])))
    }
}
