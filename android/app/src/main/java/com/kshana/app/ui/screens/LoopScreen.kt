package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Settings
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.delay
import uniffi.kshana_ffi.HourRecord
import uniffi.kshana_ffi.LifeScoreRecord
import uniffi.kshana_ffi.MomentRecord
import uniffi.kshana_ffi.ShiftRecord
import uniffi.kshana_ffi.WindowRecord
import java.time.Duration
import java.time.Instant
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter

private val CLOCK: DateTimeFormatter = DateTimeFormatter.ofPattern("HH:mm:ss")
private val LONG_DATE: DateTimeFormatter = DateTimeFormatter.ofPattern("EEEE, d MMMM")

/**
 * The time loop: what this moment holds, a countdown to the next shift, nine lives,
 * the day's hours and windows, and what is coming. Recomputes at every shift.
 */
@Composable
fun LoopScreen(
    profile: BirthProfile,
    place: Place,
    settings: EngineSettings,
    onLife: (String) -> Unit,
    onAsk: () -> Unit,
    onRemedy: () -> Unit,
    onSettings: () -> Unit,
) {
    val context = LocalContext.current
    var moment by remember { mutableStateOf<MomentRecord?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var tick by remember { mutableStateOf(Instant.now()) }

    LaunchedEffect(Unit) {
        while (true) {
            tick = Instant.now()
            delay(1000)
        }
    }
    LaunchedEffect(profile, place, settings) {
        while (true) {
            val wait = try {
                val m = loadMoment(context, profile, place, settings)
                moment = m
                error = null
                val next = m.shifts.firstOrNull()?.at?.toJava()
                next?.let { Duration.between(Instant.now(), it).toMillis() + 400 } ?: 300_000L
            } catch (e: Exception) {
                error = e.message ?: e.toString()
                5_000L
            }
            delay(wait.coerceIn(1_000L, 300_000L))
        }
    }

    val m = moment
    LazyColumn(
        Modifier.fillMaxSize(),
        contentPadding = PaddingValues(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        item {
            Column(Modifier.padding(horizontal = 24.dp)) {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                    Text("K S H A N A", fontFamily = FontFamily.Serif, fontSize = 20.sp, letterSpacing = 4.sp, color = KshanaColors.Gold)
                    Icon(Icons.Filled.Settings, contentDescription = "settings", tint = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onSettings).padding(8.dp))
                }
                val local = ZonedDateTime.ofInstant(tick, java.time.ZoneId.systemDefault())
                Text(
                    "${greeting(local.hour)}${profile.name.takeIf { it.isNotBlank() }?.let { ", ${it.substringBefore(' ')}" } ?: ""}",
                    style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text,
                )
                Row(verticalAlignment = Alignment.Bottom) {
                    Text(local.format(CLOCK), fontFamily = FontFamily.Monospace, fontSize = 30.sp, color = KshanaColors.Text)
                    Text("  ${local.format(LONG_DATE)} · ${place.name}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(bottom = 6.dp))
                }
                ErrorLine(error)
            }
        }

        item {
            if (m == null) {
                Text("reading the sky for this second…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(horizontal = 24.dp))
            } else {
                HeroCard(m, tick)
            }
        }

        if (m != null) {
            item {
                Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    SectionLabel("Your nine lives · right now")
                    LivesGrid(m.lives, onLife)
                }
            }
            item {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    SectionLabel("The hours of today", Modifier.padding(horizontal = 24.dp))
                    HoursStrip(m.hours, tick, m.lives)
                    m.hora?.let {
                        Text("Now: ${it.lord} hour · ${it.words}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.padding(horizontal = 24.dp))
                    }
                }
            }
            item {
                Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    SectionLabel("Watch the clock")
                    for (w in m.windows) WindowRow(w, tick)
                }
            }
            item {
                Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    SectionLabel("Coming up")
                    for (s in m.shifts.take(10)) ShiftRow(s, tick)
                }
            }
            item {
                Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    SectionLabel("Under the hood")
                    Text("Moon in ${m.moonStar} · ${m.taraName} tara · ${m.taraWords}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                    if (m.saturnActive) Text(m.saturnWords.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Ember)
                    Text(
                        "Running periods: " + m.dasha.joinToString(" › ") { it.lord },
                        fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Muted,
                    )
                    Spacer(Modifier.height(6.dp))
                    Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        GoldButton("ASK A QUESTION", onClick = onAsk, modifier = Modifier.weight(1f))
                        QuietButton("REMEDIES", onClick = onRemedy, modifier = Modifier.weight(1f))
                    }
                    Text("Everything is computed on this phone from your birth time. Nothing leaves it.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                }
            }
        }
    }
}

private fun greeting(hour: Int): String = when (hour) {
    in 4..11 -> "Good morning"
    in 12..16 -> "Good afternoon"
    in 17..20 -> "Good evening"
    else -> "Good night"
}

@Composable
private fun HeroCard(m: MomentRecord, tick: Instant) {
    val next = m.shifts.firstOrNull()
    Column(
        Modifier.padding(horizontal = 24.dp).fillMaxWidth().clip(RoundedCornerShape(18.dp)).background(KshanaColors.Surface).padding(18.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(18.dp)) {
            ScoreRing(m.overall, size = 92.dp, stroke = 7.dp)
            Column(Modifier.weight(1f)) {
                Text(m.headline.substringBefore(" ·"), style = MaterialTheme.typography.titleLarge, color = bandColor(m.band))
                Text(m.headline.substringAfter(" · ", ""), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                if (next != null) {
                    Spacer(Modifier.height(6.dp))
                    Text("changes in", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                    Text(countdown(tick, next.at.toJava()), fontFamily = FontFamily.Monospace, fontSize = 26.sp, color = KshanaColors.Gold)
                    Text("${hm(next.at)} · ${next.what}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
                }
            }
        }
        val top = m.lives.maxByOrNull { it.score }
        val low = m.lives.minByOrNull { it.score }
        if (top != null && low != null) {
            Text(top.headline, fontFamily = FontFamily.Serif, fontSize = 19.sp, lineHeight = 26.sp, color = KshanaColors.Text)
            Text("${lifeGlyph(top.life)} ${top.life} is your strongest now · ${lifeGlyph(low.life)} ${low.life} asks for care", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
    }
}

/** A ring filled to `score`/100 with the number inside. */
@Composable
internal fun ScoreRing(score: Double, size: androidx.compose.ui.unit.Dp, stroke: androidx.compose.ui.unit.Dp, label: String? = null) {
    val color = scoreColor(score)
    Box(Modifier.size(size), contentAlignment = Alignment.Center) {
        Canvas(Modifier.fillMaxSize()) {
            val s = stroke.toPx()
            val inset = s / 2
            val arcSize = Size(this.size.width - s, this.size.height - s)
            drawArc(KshanaColors.Line, -90f, 360f, false, Offset(inset, inset), arcSize, style = Stroke(s, cap = StrokeCap.Round))
            drawArc(color, -90f, (360.0 * (score / 100.0)).toFloat(), false, Offset(inset, inset), arcSize, style = Stroke(s, cap = StrokeCap.Round))
        }
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text(score.toInt().toString(), fontFamily = FontFamily.Monospace, fontWeight = FontWeight.Medium, fontSize = (size.value / 3.2).sp, color = KshanaColors.Text)
            if (label != null) Text(label, style = MaterialTheme.typography.labelSmall, color = color)
        }
    }
}

@Composable
private fun LivesGrid(lives: List<LifeScoreRecord>, onLife: (String) -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        for (row in lives.chunked(3)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (l in row) LifeTile(l, Modifier.weight(1f)) { onLife(l.life) }
                repeat(3 - row.size) { Spacer(Modifier.weight(1f)) }
            }
        }
    }
}

@Composable
private fun LifeTile(l: LifeScoreRecord, modifier: Modifier, onClick: () -> Unit) {
    val c = bandColor(l.band)
    Column(
        modifier.clip(RoundedCornerShape(14.dp)).background(KshanaColors.Surface).clickable(onClick = onClick).padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(lifeGlyph(l.life), fontSize = 14.sp, color = c)
            Text(l.score.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 22.sp, color = KshanaColors.Text)
        }
        Text(l.life, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
        Box(Modifier.fillMaxWidth().height(4.dp).clip(RoundedCornerShape(2.dp)).background(KshanaColors.Line)) {
            Box(Modifier.fillMaxWidth((l.score / 100.0).toFloat().coerceIn(0.02f, 1f)).height(4.dp).background(c))
        }
        Text(bandWord(l.band), style = MaterialTheme.typography.labelSmall, color = c)
    }
}

@Composable
private fun HoursStrip(hours: List<HourRecord>, tick: Instant, lives: List<LifeScoreRecord>) {
    if (hours.isEmpty()) return
    LazyRow(contentPadding = PaddingValues(horizontal = 24.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        items(hours) { h ->
            val start = h.start.toJava()
            val end = h.end.toJava()
            val now = !tick.isBefore(start) && tick.isBefore(end)
            val past = !tick.isBefore(end)
            val mean = if (h.fit.isEmpty()) 50.0 else h.fit.average()
            val bestIdx = h.fit.withIndex().maxByOrNull { it.value }?.index
            val best = bestIdx?.let { lives.getOrNull(it)?.life } ?: ""
            Column(
                Modifier.width(76.dp).clip(RoundedCornerShape(12.dp))
                    .background(if (now) KshanaColors.Gold.copy(alpha = 0.14f) else KshanaColors.Surface)
                    .then(if (now) Modifier.border(1.dp, KshanaColors.Gold, RoundedCornerShape(12.dp)) else Modifier)
                    .padding(10.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(3.dp),
            ) {
                Text(hm(h.start), fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = if (past) KshanaColors.Muted.copy(alpha = 0.5f) else KshanaColors.Muted)
                Text(h.lord, fontFamily = FontFamily.Serif, fontSize = 15.sp, color = if (past) KshanaColors.TextSoft.copy(alpha = 0.5f) else KshanaColors.Text)
                Box(Modifier.size(8.dp).clip(CircleShape).background(if (h.caution) KshanaColors.Ember else scoreColor(mean)))
                Text(if (h.caution) "pause" else best, style = MaterialTheme.typography.labelSmall, color = if (h.caution) KshanaColors.Ember else KshanaColors.Muted, textAlign = TextAlign.Center)
            }
        }
    }
}

@Composable
private fun WindowRow(w: WindowRecord, tick: Instant) {
    val active = !tick.isBefore(w.start.toJava()) && tick.isBefore(w.end.toJava())
    val c = if (w.caution) KshanaColors.Ember else KshanaColors.Leaf
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(8.dp).clip(CircleShape).background(c))
        Spacer(Modifier.width(10.dp))
        Text(w.label, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text, modifier = Modifier.weight(1f))
        Text("${hm(w.start)} – ${hm(w.end)}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = if (active) c else KshanaColors.Muted)
        if (active) Text("  now", style = MaterialTheme.typography.labelSmall, color = c)
    }
    Text(
        if (w.caution) "Finish things, start nothing new." else "The best minutes of the day to begin.",
        style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(start = 18.dp),
    )
}

@Composable
private fun ShiftRow(s: ShiftRecord, tick: Instant) {
    val c: Color = when (s.kind) {
        "kala" -> KshanaColors.Ember
        "star", "sign", "tithi" -> KshanaColors.Sky
        "dasha" -> KshanaColors.Gold
        else -> KshanaColors.Muted
    }
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(hm(s.at), fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Muted, modifier = Modifier.width(52.dp))
        Text(s.what, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
        Text("in ${countdown(tick, s.at.toJava())}", fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = c)
    }
}
