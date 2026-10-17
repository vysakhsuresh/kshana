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
import androidx.compose.foundation.lazy.rememberLazyListState
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
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import androidx.compose.material3.HorizontalDivider
import com.kshana.app.engine.EngineProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.BriefingRecord
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
    onPlan: () -> Unit = {},
    onBook: () -> Unit = {},
    onTogether: () -> Unit = {},
) {
    val context = LocalContext.current
    var moment by remember { mutableStateOf<MomentRecord?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var tick by remember { mutableStateOf(Instant.now()) }
    // Time travel: an offset from now, in seconds. Zero means live.
    var travelSeconds by remember { mutableStateOf(0L) }
    val travelling = travelSeconds != 0L

    LaunchedEffect(Unit) {
        while (true) {
            tick = Instant.now()
            delay(1000)
        }
    }
    LaunchedEffect(profile, place, settings, travelSeconds) {
        while (true) {
            val wait = try {
                val at = Instant.now().plusSeconds(travelSeconds)
                val m = loadMoment(context, profile, place, settings, Instants.fromInstant(at))
                moment = m
                error = null
                if (travelling) {
                    60_000L
                } else {
                    val next = m.shifts.firstOrNull()?.at?.toJava()
                    next?.let { Duration.between(Instant.now(), it).toMillis() + 400 } ?: 300_000L
                }
            } catch (e: Exception) {
                error = e.message ?: e.toString()
                5_000L
            }
            delay(wait.coerceIn(1_000L, 300_000L))
        }
    }
    val shownTick = tick.plusSeconds(travelSeconds)

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
                val local = ZonedDateTime.ofInstant(shownTick, java.time.ZoneId.systemDefault())
                Text(
                    "${greeting(local.hour)}${profile.name.takeIf { it.isNotBlank() }?.let { ", ${it.substringBefore(' ')}" } ?: ""}",
                    style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text,
                )
                Row(verticalAlignment = Alignment.Bottom) {
                    Text(local.format(CLOCK), fontFamily = FontFamily.Monospace, fontSize = 30.sp, color = KshanaColors.Text)
                    Text("  ${local.format(LONG_DATE)} · ${place.name}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(bottom = 6.dp))
                }
                ErrorLine(error)
                // Time travel: step the whole loop forward or back.
                Row(Modifier.fillMaxWidth().padding(top = 8.dp), horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text(if (travelling) "TRAVELLING" else "TRAVEL", style = MaterialTheme.typography.labelSmall, color = if (travelling) KshanaColors.Ember else KshanaColors.Muted, modifier = Modifier.width(72.dp))
                    for ((label, secs) in listOf("−1d" to -86_400L, "+1h" to 3_600L, "+1d" to 86_400L, "+7d" to 604_800L, "+30d" to 2_592_000L)) {
                        Text(
                            label, fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky,
                            modifier = Modifier.clip(RoundedCornerShape(10.dp)).background(KshanaColors.Surface).clickable { travelSeconds += secs; moment = null }.padding(horizontal = 10.dp, vertical = 6.dp),
                        )
                    }
                    if (travelling) {
                        Text(
                            "NOW", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Night,
                            modifier = Modifier.clip(RoundedCornerShape(10.dp)).background(KshanaColors.Gold).clickable { travelSeconds = 0L; moment = null }.padding(horizontal = 10.dp, vertical = 6.dp),
                        )
                    }
                }
            }
        }

        item {
            if (m == null) {
                Text("reading the sky for this second…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(horizontal = 24.dp))
            } else {
                HeroCard(m, shownTick, travelling)
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
                Column(Modifier.padding(horizontal = 24.dp)) {
                    BriefingCard(profile, place, settings, travelSeconds, onLife, onRemedy)
                }
            }
            item {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    SectionLabel("The hours of today", Modifier.padding(horizontal = 24.dp))
                    HoursStrip(m.hours, shownTick, m.lives)
                    m.hora?.let {
                        Text("Now: ${it.lord} hour · ${it.words}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.padding(horizontal = 24.dp))
                    }
                }
            }
            item {
                Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    SectionLabel("Watch the clock")
                    for (w in m.windows) WindowRow(w, shownTick)
                }
            }
            item {
                Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    SectionLabel("Coming up")
                    for (s in m.shifts.take(10)) ShiftRow(s, shownTick)
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
                        GoldButton("ASK", onClick = onAsk, modifier = Modifier.weight(1f))
                        QuietButton("REMEDIES", onClick = onRemedy, modifier = Modifier.weight(1f))
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        QuietButton("PLAN A TIME", onClick = onPlan, modifier = Modifier.weight(1f))
                        QuietButton("THE TWO OF US", onClick = onTogether, modifier = Modifier.weight(1f))
                    }
                    QuietButton("THE BOOK OF YOUR LIFE · PAST, PRESENT, FUTURE", onClick = onBook, modifier = Modifier.fillMaxWidth())
                    Text("Everything is computed on this phone from your birth time. Nothing leaves it.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                }
            }
        }
    }
}

/** One briefing per profile, place and hour is kept for the process so the Loop stays instant. */
internal object BriefingCache {
    var key: String? = null
    var record: BriefingRecord? = null
}

/**
 * The reading, as an astrologer would give it: the birth Moon (so it can be checked),
 * the running periods with their ends, Saturn's phase and its exit date, the moment in
 * words, the days to circle this month and the remedy in brief. Loads once per hour.
 */
@Composable
private fun BriefingCard(
    profile: BirthProfile,
    place: Place,
    settings: EngineSettings,
    travelSeconds: Long,
    onLife: (String) -> Unit,
    onRemedy: () -> Unit,
) {
    val context = LocalContext.current
    val hourKey = (System.currentTimeMillis() + travelSeconds * 1000L) / 3_600_000L
    val key = "${profile.birthInstant().taiSeconds}|${place.name}|${settings.ayanamsa}|${settings.houseSystem}|$hourKey"
    var b by remember { mutableStateOf(if (BriefingCache.key == key) BriefingCache.record else null) }
    var error by remember { mutableStateOf<String?>(null) }
    var open by remember { mutableStateOf(false) }
    LaunchedEffect(key) {
        if (BriefingCache.key == key && BriefingCache.record != null) { b = BriefingCache.record; return@LaunchedEffect }
        try {
            val engine = EngineProvider.get(context)
            val at = Instants.fromInstant(Instant.now().plusSeconds(travelSeconds))
            val r = withContext(Dispatchers.Default) {
                engine.briefing(
                    profile.birthInstant(), profile.latitude, profile.longitude,
                    place.latitude, place.longitude,
                    settings.houseSystem, settings.ayanamsa, listOf("Love", "Career", "Money", "Health"), at,
                )
            }
            BriefingCache.key = key
            BriefingCache.record = r
            b = r
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }
    Column(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(18.dp)).background(KshanaColors.Surface).padding(18.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("YOUR READING · AS AN ASTROLOGER WOULD SAY IT", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        ErrorLine(error)
        val r = b
        if (r == null) {
            if (error == null) Text("reading your chart aloud…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            return@Column
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(14.dp)) {
            Column(Modifier.weight(1f)) {
                Text("BIRTH MOON", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                Text(r.moon, fontFamily = FontFamily.Serif, fontSize = 17.sp, color = KshanaColors.Text)
            }
            Column(Modifier.weight(1.4f)) {
                Text("RUNNING", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                Text(r.dasha.replace(" (to ", "\nto ").replace(")", ""), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
            }
        }
        Text(r.saturnWords + ".", style = MaterialTheme.typography.bodyMedium, color = if (r.saturnActive) KshanaColors.Ember else KshanaColors.TextSoft)
        val shown = if (open) r.paragraphs else r.paragraphs.drop(2).take(1)
        for (p in shown) Text(p, fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 25.sp, color = KshanaColors.Text)
        Text(
            if (open) "LESS −" else "THE WHOLE READING +",
            style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
            modifier = Modifier.clickable { open = !open }.padding(vertical = 2.dp),
        )
        if (open) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            Text("DAYS TO CIRCLE · THE NEXT THIRTY", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            for (c in r.circled) {
                Row(Modifier.fillMaxWidth().clickable { onLife(c.life) }.padding(vertical = 3.dp), verticalAlignment = Alignment.Top) {
                    Text(lifeGlyph(c.life), fontSize = 14.sp, color = scoreColor(c.score), modifier = Modifier.width(22.dp).padding(top = 2.dp))
                    Column(Modifier.weight(1f)) {
                        Text("${c.life} · ${localText(c.at, WEEKDAY_DAY)}", style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text)
                        Text(c.words.substringAfter(". ", c.words), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                    }
                    Text(c.score.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 16.sp, color = scoreColor(c.score))
                }
            }
            r.remedy?.let { rem ->
                HorizontalDivider(color = KshanaColors.LineSoft)
                Text("THE REMEDY IN BRIEF · ${rem.planet.uppercase()}", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text(rem.cause, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                for (t in rem.top) {
                    Row(Modifier.padding(vertical = 2.dp), verticalAlignment = Alignment.Top) {
                        Text(t.kind.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.width(78.dp).padding(top = 3.dp))
                        Column(Modifier.weight(1f)) {
                            Text(t.title, style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text)
                            Text(t.why, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                        }
                    }
                }
                QuietButton("ALL ${rem.total} REMEDIES", onClick = onRemedy, modifier = Modifier.fillMaxWidth())
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
private fun HeroCard(m: MomentRecord, tick: Instant, travelling: Boolean) {
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
                if (travelling) {
                    Spacer(Modifier.height(6.dp))
                    Text("you are looking at", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Ember)
                    Text(localText(m.at, DATE_TIME), fontFamily = FontFamily.Monospace, fontSize = 18.sp, color = KshanaColors.Gold)
                    if (next != null) Text("then: ${hm(next.at)} · ${next.what}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
                } else if (next != null) {
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
    val state = rememberLazyListState()
    val currentIndex = hours.indexOfFirst { !tick.isBefore(it.start.toJava()) && tick.isBefore(it.end.toJava()) }
    LaunchedEffect(hours.firstOrNull()?.start, currentIndex) {
        if (currentIndex >= 0) state.animateScrollToItem((currentIndex - 1).coerceAtLeast(0))
    }
    LazyRow(state = state, contentPadding = PaddingValues(horizontal = 24.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        items(hours) { h ->
            val start = h.start.toJava()
            val end = h.end.toJava()
            val now = !tick.isBefore(start) && tick.isBefore(end)
            val past = !tick.isBefore(end)
            val mean = if (h.fit.isEmpty()) 50.0 else h.fit.average()
            val bestIdx = h.fit.withIndex().maxByOrNull { it.value }?.index
            val best = bestIdx?.let { lives.getOrNull(it)?.life } ?: ""
            Column(
                Modifier.width(92.dp).clip(RoundedCornerShape(12.dp))
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
    val past = !tick.isBefore(w.end.toJava())
    val alpha = if (past) 0.45f else 1f
    val c = if (w.caution) KshanaColors.Ember else KshanaColors.Leaf
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(8.dp).clip(CircleShape).background(c.copy(alpha = alpha)))
        Spacer(Modifier.width(10.dp))
        Text(w.label, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text.copy(alpha = alpha), modifier = Modifier.weight(1f))
        Text("${hm(w.start)} – ${hm(w.end)}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = if (active) c else KshanaColors.Muted.copy(alpha = alpha))
        if (active) Text("  now", style = MaterialTheme.typography.labelSmall, color = c)
        if (past) Text("  done", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted.copy(alpha = alpha))
    }
    if (!past) {
        Text(
            if (w.caution) "Finish things, start nothing new." else "The best minutes of the day to begin.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(start = 18.dp),
        )
    }
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
