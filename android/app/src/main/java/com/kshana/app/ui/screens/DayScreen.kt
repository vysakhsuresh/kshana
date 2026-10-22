package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowRight
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.sky.BreathingRing
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.DayClockRecord
import uniffi.kshana_ffi.DayWindowRecord
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.LunarWindowRecord
import uniffi.kshana_ffi.PakshiRecord
import uniffi.kshana_ffi.SpecialLagnaRecord
import uniffi.kshana_ffi.UpagrahaRecord
import uniffi.kshana_ffi.birthBird
import java.time.DateTimeException
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import kotlin.math.floor
import com.kshana.app.ui.sky.KshanaFonts

private val DAY_TITLE = DateTimeFormatter.ofPattern("EEE d MMM uuuu")
private val HMS_MS = DateTimeFormatter.ofPattern("HH:mm:ss.SSS")
private val HMS = DateTimeFormatter.ofPattern("HH:mm:ss")
private val SHORT_TIME = DateTimeFormatter.ofPattern("HH:mm")

/** The Moon's windows are listed this many days ahead of today. */
private const val MOON_DAYS = 31L

/* The day bar: three lanes of equal height under a strip for the needle's head. */
private const val BAR_TOP_DP = 8
private const val LANE_H_DP = 16
private const val LANE_GAP_DP = 3
private const val BAR_BOTTOM_DP = 4

/** What a tap on the bar, or a row in a list, picked. */
private sealed class Picked {
    data class Window(val w: DayWindowRecord) : Picked()
    data class Bird(val p: PakshiRecord) : Picked()
}

/** The chips of the Moon's windows: the engine's kind, the label, and whether a natal Moon is needed. */
private enum class MoonKind(val kind: String, val label: String, val natal: Boolean) {
    Gandanta("gandanta", "Gandanta", false),
    Panchaka("panchaka", "Panchaka", false),
    Bhadra("bhadra", "Bhadra", false),
    Yoga("yoga", "Inauspicious yogas", false),
    Tara("tara", "Tara", true),
    Chandra("chandra", "Chandra", true),
}

/** What the engine solved for one solar day. */
private class DaySolve(val clock: DayClockRecord, val pakshi: List<PakshiRecord>)

/** A ghati reading: whole ghatis and vighatis elapsed, and the vipala within the vighati. */
private class GhatiReading(val ghati: Int, val vighati: Int, val vipala: Double)

/**
 * The day's clocks: the solved sunrise and the sixty ghatis that run from it, the thirty
 * muhurtas, the sixteen choghadiya, the ten Pancha Pakshi windows, the upagrahas and the
 * special lagnas, then the Moon's windows for the month ahead.
 */
@Composable
fun DayScreen(profile: BirthProfile?, here: Place, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val zone = remember(here, profile) {
        try { com.kshana.app.data.CityClocks.of(here.zoneId) } catch (e: DateTimeException) { profile?.zone() ?: ZoneId.systemDefault() }
    }

    // The clock: ticks every second. Only the composables that read it recompose at that rate.
    val nowState = remember { mutableStateOf(System.currentTimeMillis()) }
    LaunchedEffect(Unit) {
        while (true) {
            delay(1_000)
            nowState.value = System.currentTimeMillis()
        }
    }
    val today by remember(zone) { derivedStateOf { Instant.ofEpochMilli(nowState.value).atZone(zone).toLocalDate() } }
    var date by remember(zone) { mutableStateOf(LocalDate.now(zone)) }
    val isToday = date == today

    var error by remember { mutableStateOf<String?>(null) }

    // The natal Moon and the bird of birth, once per person.
    var natalMoon by remember(profile, settings, com.kshana.app.data.Lang.code) { mutableStateOf<Double?>(null) }
    var bird by remember(profile, settings, com.kshana.app.data.Lang.code) { mutableStateOf<String?>(null) }
    LaunchedEffect(profile, settings, com.kshana.app.data.Lang.code) {
        if (profile == null) return@LaunchedEffect
        try {
            val engine = EngineProvider.get(context)
            val chart = withContext(Dispatchers.Default) {
                engine.chart(profile.birthInstant(), profile.latitude, profile.longitude, 0.0, settings.houseSystem, settings.ayanamsa, settings.topocentricMoon)
            }
            val moon = chart.panchanga.moonSidereal
            natalMoon = moon
            bird = birthBird(moon, chart.panchanga.tithiNumber.toInt() <= 15, com.kshana.app.data.Lang.code)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    // The day's clocks, solved once per day and kept. Today is solved at now; another day at its noon.
    val cache = remember(here, settings, profile, com.kshana.app.data.Lang.code) { mutableStateMapOf<LocalDate, DaySolve>() }
    val solve = cache[date]
    val stale = isToday && solve != null && nowState.value >= millisOf(solve.clock.nextSunrise)
    LaunchedEffect(date, here, settings, profile, stale, com.kshana.app.data.Lang.code) {
        if (stale) cache.remove(date)
        if (cache.containsKey(date)) return@LaunchedEffect
        try {
            val engine = EngineProvider.get(context)
            val at = if (date == today) Instants.now() else Instants.fromInstant(date.atTime(12, 0).atZone(zone).toInstant())
            val solved = withContext(Dispatchers.Default) {
                val clock = engine.dayClock(at, here.latitude, here.longitude, settings.ayanamsa, com.kshana.app.data.Lang.code)
                val birds = if (profile == null) emptyList() else runCatching {
                    engine.panchaPakshi(profile.birthInstant(), settings.ayanamsa, at, here.latitude, here.longitude, com.kshana.app.data.Lang.code)
                }.getOrDefault(emptyList())
                DaySolve(clock, birds)
            }
            cache[date] = solved
            error = null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    // The Moon's windows for the month from today, one kind at a time, kept per kind.
    var moonKind by remember { mutableStateOf(MoonKind.Gandanta) }
    var moonError by remember { mutableStateOf<String?>(null) }
    val moonCache = remember(here, settings, natalMoon, today, com.kshana.app.data.Lang.code) { mutableStateMapOf<MoonKind, List<LunarWindowRecord>>() }
    LaunchedEffect(moonKind, moonCache) {
        if (moonCache.containsKey(moonKind)) return@LaunchedEffect
        if (moonKind.natal && natalMoon == null) return@LaunchedEffect
        try {
            val engine = EngineProvider.get(context)
            val from = Instants.fromInstant(today.atStartOfDay(zone).toInstant())
            val to = Instants.fromInstant(today.plusDays(MOON_DAYS).atStartOfDay(zone).toInstant())
            val found = withContext(Dispatchers.Default) {
                engine.moonWindows(moonKind.kind, settings.ayanamsa, natalMoon ?: 0.0, from, to, com.kshana.app.data.Lang.code)
            }
            moonCache[moonKind] = found.sortedBy { it.start.seconds() }
            moonError = null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            moonError = e.message ?: e.toString()
        }
    }

    var picked by remember(date) { mutableStateOf<Picked?>(null) }
    var openMuhurtas by remember { mutableStateOf(false) }
    var openChoghadiya by remember { mutableStateOf(false) }
    var openUpagrahas by remember { mutableStateOf(false) }
    var openLagnas by remember { mutableStateOf(false) }
    var openPakshi by remember { mutableStateOf(false) }

    // Now on the engine's own scale: TAI seconds, derived from the solved sunrise so no call is made per tick.
    val sunriseMs = solve?.let { millisOf(it.clock.sunrise) } ?: 0L
    val nowTai: Double = solve?.let { it.clock.sunrise.seconds() + (nowState.value - sunriseMs) / 1000.0 } ?: 0.0

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("THE DAY · ${here.name.uppercase()}", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("The day's clocks", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "Sixty ghatis from the solved sunrise, thirty muhurtas, sixteen choghadiya, your bird's ten windows: every edge to the second.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            QuietButton("‹", onClick = { date = date.minusDays(1) })
            Text(
                date.format(DAY_TITLE),
                style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text,
                textAlign = TextAlign.Center, modifier = Modifier.weight(1f),
            )
            QuietButton("›", onClick = { date = date.plusDays(1) })
            Chip("Today", selected = isToday, onClick = { date = today })
        }
        ErrorLine(error)

        if (solve == null) {
            if (error == null) Text("Solving the sunrise and the day's edges…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        } else {
            val c = solve.clock
            Text(
                "sunrise ${localText(c.sunrise, HMS_MS, zone)} · sunset ${localText(c.sunset, HMS_MS, zone)} · next sunrise ${localText(c.nextSunrise, HMS_MS, zone)}",
                fontFamily = KshanaFonts.mono, fontSize = 11.sp, lineHeight = 16.sp, color = KshanaColors.Sky,
            )
            Text(
                "The day of ${c.weekdayLord}. Brahma muhurta ${localText(c.brahmaStart, HMS, zone)} – ${localText(c.brahmaEnd, HMS, zone)}; Abhijit ${localText(c.abhijitStart, HMS, zone)} – ${localText(c.abhijitEnd, HMS, zone)}.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
            )

            if (isToday) {
                SectionLabel("The ghati clock")
                GhatiClock(c, nowState)
                RunningLine(c, solve.pakshi, nowTai)
            }

            SectionLabel("The day bar")
            ClockBar(c, solve.pakshi, nowTai, picked) { hit -> picked = if (hit == picked) null else hit }
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text("rise ${localText(c.sunrise, SHORT_TIME, zone)}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
                Text("set ${localText(c.sunset, SHORT_TIME, zone)}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
                Text("rise ${localText(c.nextSunrise, SHORT_TIME, zone)}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
            }
            Text(
                if (solve.pakshi.isEmpty()) "lanes: muhurtas · choghadiya — sky favourable, muted avoid, gold Abhijit, hatched Brahma muhurta. Tap a segment."
                else "lanes: muhurtas · choghadiya · Pancha Pakshi — sky favourable, muted avoid, gold Abhijit and your bird's good windows, hatched Brahma muhurta. Tap a segment.",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
            )
            picked?.let { PickedCard(it, zone, nowTai) { picked = null } }

            Toggle("All 30 muhurtas", openMuhurtas) { openMuhurtas = !openMuhurtas }
            if (openMuhurtas) for (w in c.muhurtas) WindowLine(w, zone, nowTai) { picked = Picked.Window(w) }

            Toggle("All choghadiya", openChoghadiya) { openChoghadiya = !openChoghadiya }
            if (openChoghadiya) for (w in c.choghadiyas) WindowLine(w, zone, nowTai) { picked = Picked.Window(w) }

            Toggle("Upagrahas", openUpagrahas) { openUpagrahas = !openUpagrahas }
            if (openUpagrahas) {
                if (c.upagrahas.isEmpty()) Text("No upagraha could be placed for this day.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                for (u in c.upagrahas) UpagrahaLine(u, zone)
            }

            Toggle("Special lagnas", openLagnas) { openLagnas = !openLagnas }
            if (openLagnas) {
                Text(
                    if (isToday) "Where each clock-lagna stood when this day was solved; they move with the ghatis." else "At this day's noon.",
                    style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
                )
                for (l in c.specialLagnas) LagnaLine(l)
            }

            if (profile != null) {
                Toggle("Pancha Pakshi", openPakshi) { openPakshi = !openPakshi }
                if (openPakshi) {
                    Text(
                        "Your bird is the ${bird ?: "…"}: the natal Moon's nakshatra in its paksha.",
                        fontFamily = KshanaFonts.serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Text,
                    )
                    if (solve.pakshi.isEmpty()) Text("The ten windows could not be laid out for this day.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                    for (p in solve.pakshi) PakshiLine(p, zone, nowTai) { picked = Picked.Bird(p) }
                }
            } else {
                Text("Add a birth to see your bird's ten windows.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(top = 6.dp))
            }
        }

        SectionLabel("The Moon's windows this month")
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (k in MoonKind.values()) {
                if (k.natal && profile == null) continue
                Chip(k.label.uppercase(), selected = moonKind == k, onClick = { moonKind = k })
            }
        }
        Text(moonBlurb(moonKind), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        if (profile == null) Text("Tara and chandra need a natal Moon; add a birth to see them.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        ErrorLine(moonError)
        val windows = moonCache[moonKind]
        when {
            windows == null -> if (moonError == null) Text(
                if (moonKind.natal && natalMoon == null) "Placing your natal Moon…" else "Solving the Moon's edges for $MOON_DAYS days…",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
            windows.isEmpty() -> Text("None in the next $MOON_DAYS days.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            else -> for (w in windows) MoonLine(w, zone, nowTai)
        }

        Spacer(Modifier.height(12.dp))
        Text(
            "Every edge is the solved sunrise or a solved root · thirty muhurtas per Muhurta Chintamani · the Pancha Pakshi matrix is marked unverified by the engine",
            fontFamily = KshanaFonts.serif, fontStyle = FontStyle.Italic, fontSize = 14.sp, lineHeight = 20.sp, color = KshanaColors.Muted,
        )
    }
}

/* ---------------------------------------------------------------------------------------
 * The ghati clock
 * ------------------------------------------------------------------------------------- */

/** Whole ghatis and vighatis since sunrise, and the vipala, for a ghati of `ghatiSeconds`. */
private fun ghatiReading(elapsedSeconds: Double, ghatiSeconds: Double): GhatiReading {
    val e = elapsedSeconds.coerceAtLeast(0.0)
    val g = floor(e / ghatiSeconds)
    val vighatiSeconds = ghatiSeconds / 60.0
    val afterGhati = e - g * ghatiSeconds
    val v = floor(afterGhati / vighatiSeconds)
    val afterVighati = afterGhati - v * vighatiSeconds
    return GhatiReading(g.toInt(), v.toInt(), afterVighati / (vighatiSeconds / 60.0))
}

/**
 * Ticks every second from the solved sunrise: sixty equal ghatis to the next sunrise on the
 * left, the fixed twenty-four-minute ghatis on the right. Nothing is asked of the engine.
 */
@Composable
private fun GhatiClock(clock: DayClockRecord, nowState: State<Long>) {
    val sunriseMs = remember(clock) { millisOf(clock.sunrise) }
    val nextMs = remember(clock) { millisOf(clock.nextSunrise) }
    val dayLength = (nextMs - sunriseMs) / 1000.0
    val elapsed = (nowState.value - sunriseMs) / 1000.0
    if (dayLength <= 0.0 || elapsed < 0.0 || elapsed > dayLength) {
        Text("The clock runs from sunrise to the next sunrise; this moment is outside the solved day.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        return
    }
    val solar = ghatiReading(elapsed, dayLength / 60.0)
    val fixed = ghatiReading(elapsed, 1440.0)
    val shape = RoundedCornerShape(8.dp)
    Row(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(16.dp),
        horizontalArrangement = Arrangement.spacedBy(14.dp), verticalAlignment = Alignment.CenterVertically,
    ) {
        BreathingRing(progress = (elapsed / dayLength).toFloat(), modifier = Modifier.size(44.dp))
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Text(
                "ghati ${solar.ghati} · vighati ${solar.vighati} · vipala ${"%.1f".format(solar.vipala)}",
                fontFamily = KshanaFonts.mono, fontSize = 15.sp, color = KshanaColors.Gold,
            )
            Text(
                "SIXTY EQUAL GHATIS, SUNRISE TO NEXT SUNRISE · ONE GHATI = ${"%.1f".format(dayLength / 60.0 / 60.0)} MIN",
                style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted,
            )
            Text(
                "fixed ghati ${fixed.ghati} · vighati ${fixed.vighati} · vipala ${"%.1f".format(fixed.vipala)}",
                fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky,
            )
            Text(
                "FIXED 24-MINUTE GHATIS FROM THE SAME SUNRISE (SAVANA)",
                style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted,
            )
        }
    }
}

/** The muhurta, the choghadiya and the bird's window running at `nowTai`. */
@Composable
private fun RunningLine(clock: DayClockRecord, pakshi: List<PakshiRecord>, nowTai: Double) {
    val m = clock.muhurtas.firstOrNull { it.start.seconds() <= nowTai && nowTai < it.end.seconds() }
    val ch = clock.choghadiyas.firstOrNull { it.start.seconds() <= nowTai && nowTai < it.end.seconds() }
    val p = pakshi.firstOrNull { it.start.seconds() <= nowTai && nowTai < it.end.seconds() }
    val parts = listOfNotNull(
        m?.let { "muhurta ${it.index.toInt() + 1} ${it.name}" },
        ch?.let { "choghadiya ${it.name}" },
        p?.let { "${it.bird} ${if (com.kshana.app.data.Lang.code == "en") it.activity.lowercase() else it.activity}" },
    )
    if (parts.isNotEmpty()) Text("now · " + parts.joinToString(" · "), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
}

/* ---------------------------------------------------------------------------------------
 * The day bar
 * ------------------------------------------------------------------------------------- */

/**
 * Sunrise to next sunrise in up to three lanes: the thirty muhurtas, the sixteen choghadiya,
 * the ten Pancha Pakshi windows. Favourable in sky, avoid in muted, Abhijit and the bird's good
 * windows in gold, Brahma muhurta hatched in gold, a gold needle at now. A tap picks a segment.
 */
@Composable
private fun ClockBar(clock: DayClockRecord, pakshi: List<PakshiRecord>, nowTai: Double, picked: Picked?, onPick: (Picked?) -> Unit) {
    val lanes = if (pakshi.isEmpty()) 2 else 3
    val totalDp = BAR_TOP_DP + lanes * LANE_H_DP + (lanes - 1) * LANE_GAP_DP + BAR_BOTTOM_DP
    val s0 = clock.sunrise.seconds()
    val span = clock.nextSunrise.seconds() - s0
    Canvas(
        Modifier.fillMaxWidth().height(totalDp.dp).pointerInput(clock, pakshi) {
            detectTapGestures { off ->
                if (span <= 0.0 || size.width <= 0) return@detectTapGestures
                val t = s0 + (off.x / size.width).toDouble() * span
                val top = BAR_TOP_DP.dp.toPx()
                val pitch = LANE_H_DP.dp.toPx() + LANE_GAP_DP.dp.toPx()
                if (off.y < top) return@detectTapGestures
                val lane = ((off.y - top) / pitch).toInt()
                if (lane < 0 || lane >= lanes) return@detectTapGestures
                val hit: Picked? = when (lane) {
                    0 -> clock.muhurtas.firstOrNull { it.start.seconds() <= t && t < it.end.seconds() }?.let { Picked.Window(it) }
                    1 -> clock.choghadiyas.firstOrNull { it.start.seconds() <= t && t < it.end.seconds() }?.let { Picked.Window(it) }
                    else -> pakshi.firstOrNull { it.start.seconds() <= t && t < it.end.seconds() }?.let { Picked.Bird(it) }
                }
                onPick(hit)
            }
        },
    ) {
        if (span <= 0.0) return@Canvas
        val w = size.width
        val top = BAR_TOP_DP.dp.toPx()
        val laneH = LANE_H_DP.dp.toPx()
        val gap = LANE_GAP_DP.dp.toPx()
        val bottom = top + lanes * laneH + (lanes - 1) * gap
        fun x(e: ExactInstant): Float = progress(clock.sunrise, clock.nextSunrise, e) * w
        fun laneTop(i: Int): Float = top + i * (laneH + gap)

        // The ground of each lane, with the night a shade darker.
        val xs = x(clock.sunset)
        for (i in 0 until lanes) {
            drawRect(KshanaColors.LineSoft, topLeft = Offset(0f, laneTop(i)), size = Size(w, laneH))
            drawRect(KshanaColors.Night, topLeft = Offset(xs, laneTop(i)), size = Size(w - xs, laneH))
        }

        fun segment(lane: Int, start: ExactInstant, end: ExactInstant, colour: Color, selected: Boolean) {
            val a = x(start)
            val b = x(end)
            val ty = laneTop(lane)
            val sw = (b - a).coerceAtLeast(1f)
            drawRect(colour, topLeft = Offset(a, ty), size = Size(sw, laneH))
            drawLine(KshanaColors.Night, Offset(b, ty), Offset(b, ty + laneH), 1f)
            if (selected) drawRect(KshanaColors.Gold, topLeft = Offset(a, ty), size = Size(sw, laneH), style = Stroke(1.5.dp.toPx()))
        }

        for (m in clock.muhurtas) {
            segment(0, m.start, m.end, if (m.favourable) KshanaColors.Sky.copy(alpha = 0.55f) else KshanaColors.Muted.copy(alpha = 0.3f), picked == Picked.Window(m))
        }
        segment(0, clock.abhijitStart, clock.abhijitEnd, KshanaColors.Gold.copy(alpha = 0.85f), false)
        for (c in clock.choghadiyas) {
            segment(1, c.start, c.end, if (c.favourable) KshanaColors.Sky.copy(alpha = 0.55f) else KshanaColors.Muted.copy(alpha = 0.3f), picked == Picked.Window(c))
        }
        for (p in pakshi) {
            val alpha = 0.25f + 0.65f * p.strength.coerceIn(0.0, 1.0).toFloat()
            segment(2, p.start, p.end, (if (p.favourable) KshanaColors.Gold else KshanaColors.Muted).copy(alpha = alpha), picked == Picked.Bird(p))
        }

        // Brahma muhurta: gold hatching across every lane.
        val ba = x(clock.brahmaStart)
        val bb = x(clock.brahmaEnd)
        if (bb - ba > 1f) {
            val rise = bottom - top
            val step = 5.dp.toPx()
            clipRect(ba, top, bb, bottom) {
                var sx = ba - rise
                while (sx < bb) {
                    drawLine(KshanaColors.Gold.copy(alpha = 0.8f), Offset(sx, bottom), Offset(sx + rise, top), 1f)
                    sx += step
                }
            }
            drawRect(KshanaColors.Gold.copy(alpha = 0.8f), topLeft = Offset(ba, top), size = Size(bb - ba, rise), style = Stroke(1f))
        }

        // Sunset, lane outlines.
        drawLine(KshanaColors.Muted.copy(alpha = 0.6f), Offset(xs, top), Offset(xs, bottom), 1f)
        for (i in 0 until lanes) drawRect(KshanaColors.Line, topLeft = Offset(0f, laneTop(i)), size = Size(w, laneH), style = Stroke(1f))

        // The needle, when now is inside the solved day.
        val nf = ((nowTai - s0) / span)
        if (nf in 0.0..1.0) {
            val xn = (nf * w).toFloat()
            drawLine(KshanaColors.Gold, Offset(xn, 0f), Offset(xn, size.height), 2f)
            val tri = Path().apply {
                moveTo(xn - 5.dp.toPx(), 0f)
                lineTo(xn + 5.dp.toPx(), 0f)
                lineTo(xn, top - 1f)
                close()
            }
            drawPath(tri, KshanaColors.Gold)
        }
    }
}

/** The picked segment: its name, its edges to the second, its verdict, a reminder at its start. */
@Composable
private fun PickedCard(p: Picked, zone: ZoneId, nowTai: Double, onClose: () -> Unit) {
    val shape = RoundedCornerShape(8.dp)
    val start: ExactInstant
    val end: ExactInstant
    val title: String
    val sub: String
    val favourable: Boolean
    val key: String
    when (p) {
        is Picked.Window -> {
            start = p.w.start; end = p.w.end
            title = p.w.name
            val lord = if (p.w.lord.isBlank()) null else if (p.w.kind == "choghadiya") "lord ${p.w.lord}" else "also ${p.w.lord}"
            sub = listOfNotNull("${p.w.kind} ${p.w.index.toInt() + 1}", lord, if (p.w.night) "night" else "day").joinToString(" · ")
            favourable = p.w.favourable
            key = "day:${p.w.kind}:${p.w.start.taiSeconds}"
        }
        is Picked.Bird -> {
            start = p.p.start; end = p.p.end
            title = "${p.p.bird} · ${p.p.activity}"
            sub = "yama ${p.p.yama.toInt()} · ${if (p.p.night) "night" else "day"} · strength ${"%.2f".format(p.p.strength)} · Pancha Pakshi"
            favourable = p.p.favourable
            key = "day:pakshi:${p.p.start.taiSeconds}"
        }
    }
    val live = start.seconds() <= nowTai && nowTai < end.seconds()
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, if (favourable) KshanaColors.Gold else KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.Top) {
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(title, style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text)
                Text(sub, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            }
            Text("CLOSE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onClose).padding(8.dp))
        }
        Text(spanText(start, end, zone), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky)
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text(
                (if (favourable) "FAVOURABLE" else "AVOID") + if (live) " · NOW" else "",
                style = MaterialTheme.typography.labelSmall, color = if (favourable) KshanaColors.Gold else KshanaColors.Muted,
            )
            RemindButton(
                key = key, atMillis = millisOf(start),
                title = "$title begins", text = "${localText(start, HMS, zone)} – ${localText(end, HMS, zone)} · ${if (favourable) "favourable" else "avoid"}",
            )
        }
    }
}

/* ---------------------------------------------------------------------------------------
 * The lists
 * ------------------------------------------------------------------------------------- */

/** A gold heading that opens and closes a list. */
@Composable
private fun Toggle(label: String, open: Boolean, onToggle: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().heightIn(min = 44.dp).clickable(onClick = onToggle).padding(top = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(label.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.weight(1f))
        Icon(Icons.Filled.KeyboardArrowRight, contentDescription = null, tint = KshanaColors.Gold, modifier = Modifier.size(18.dp).rotate(if (open) 90f else 0f))
    }
}

/** One muhurta or choghadiya: its number, its edges, its name, its verdict. */
@Composable
private fun WindowLine(w: DayWindowRecord, zone: ZoneId, nowTai: Double, onPick: () -> Unit) {
    val live = w.start.seconds() <= nowTai && nowTai < w.end.seconds()
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(Modifier.fillMaxWidth().clickable(onClick = onPick).padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
        Text("${w.index.toInt() + 1}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted, modifier = Modifier.width(22.dp))
        Text(
            "${localText(w.start, HMS, zone)} – ${localText(w.end, HMS, zone)}",
            fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = if (live) KshanaColors.Gold else KshanaColors.Sky,
        )
        Text(
            w.name + (if (w.lord.isBlank()) "" else " · ${w.lord}") + (if (w.night) " · night" else ""),
            style = MaterialTheme.typography.bodyMedium, color = if (w.favourable) KshanaColors.TextSoft else KshanaColors.Muted,
            modifier = Modifier.weight(1f).padding(start = 12.dp),
        )
        Text(
            if (live) "NOW" else if (w.favourable) "GOOD" else "AVOID",
            style = MaterialTheme.typography.labelSmall, color = if (live || w.favourable) KshanaColors.Gold else KshanaColors.Muted,
        )
    }
}

/** One upagraha: its name and point, its sidereal place, the kala instant it stands for. */
@Composable
private fun UpagrahaLine(u: UpagrahaRecord, zone: ZoneId) {
    HorizontalDivider(color = KshanaColors.LineSoft)
    Column(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(u.name, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text, modifier = Modifier.weight(1f))
            Text("${SIGN_SHORT[signIndex(u.sidereal)]} ${dms(u.sidereal)}", fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky)
        }
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("${u.point} · ${if (u.night) "night" else "day"} kala", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.weight(1f))
            Text(localText(u.at, DATE_TIME_S, zone), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
        }
    }
}

/** One special lagna: its sidereal place and, in plain words, how it moves. */
@Composable
private fun LagnaLine(l: SpecialLagnaRecord) {
    HorizontalDivider(color = KshanaColors.LineSoft)
    Column(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(l.name, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text, modifier = Modifier.weight(1f))
            Text("${signName(l.sidereal)} ${dms(l.sidereal)}", fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky)
        }
        Text(lagnaWords(l.key), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    }
}

/** One Pancha Pakshi window: its yama, its edges, the bird and what it does, its strength. */
@Composable
private fun PakshiLine(p: PakshiRecord, zone: ZoneId, nowTai: Double, onPick: () -> Unit) {
    val live = p.start.seconds() <= nowTai && nowTai < p.end.seconds()
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(Modifier.fillMaxWidth().clickable(onClick = onPick).padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
        Text("${if (p.night) "n" else "d"}${p.yama.toInt()}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted, modifier = Modifier.width(22.dp))
        Text(
            "${localText(p.start, HMS, zone)} – ${localText(p.end, HMS, zone)}",
            fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = if (live) KshanaColors.Gold else KshanaColors.Sky,
        )
        Text(
            "${p.bird} ${if (com.kshana.app.data.Lang.code == "en") p.activity.lowercase() + "s" else "· " + p.activity} · ${"%.0f".format(p.strength * 100.0)}%",
            style = MaterialTheme.typography.bodyMedium, color = if (p.favourable) KshanaColors.TextSoft else KshanaColors.Muted,
            modifier = Modifier.weight(1f).padding(start = 12.dp),
        )
        Text(
            if (live) "NOW" else if (p.favourable) "GOOD" else "AVOID",
            style = MaterialTheme.typography.labelSmall, color = if (live || p.favourable) KshanaColors.Gold else KshanaColors.Muted,
        )
    }
}

/** One of the Moon's windows: its label, its edges to the second, its verdict. */
@Composable
private fun MoonLine(w: LunarWindowRecord, zone: ZoneId, nowTai: Double) {
    val live = w.start.seconds() <= nowTai && nowTai < w.end.seconds()
    HorizontalDivider(color = KshanaColors.LineSoft)
    Column(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(w.label, style = MaterialTheme.typography.bodyMedium, color = if (w.favourable) KshanaColors.Gold else KshanaColors.Muted, modifier = Modifier.weight(1f))
            Text(
                if (live) "NOW" else if (w.favourable) "FAVOURABLE" else "AVOID",
                style = MaterialTheme.typography.labelSmall, color = if (live || w.favourable) KshanaColors.Gold else KshanaColors.Muted,
            )
        }
        Text(spanText(w.start, w.end, zone), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = if (live) KshanaColors.Gold else KshanaColors.Sky)
    }
}

/* ---------------------------------------------------------------------------------------
 * Words and helpers
 * ------------------------------------------------------------------------------------- */

private fun lagnaWords(name: String): String {
    val n = name.lowercase()
    return when {
        n.startsWith("bhava") -> "The Sun's place at sunrise, moved on one sign every five ghatis: two hours a sign."
        n.startsWith("hora") -> "The Sun's place at sunrise, moved on one sign every two and a half ghatis: an hour a sign."
        n.startsWith("ghati") -> "The Sun's place at sunrise, moved on one sign every ghati: twenty-four minutes a sign."
        n.startsWith("vighati") -> "The Sun's place at sunrise, moved on one sign every vighati: the fastest of the clocks."
        n.startsWith("pranapada") -> "The Sun's place moved on one sign every fifteen vighatis, then shifted by the nature of the Sun's sign: movable, fixed or dual."
        n.startsWith("sree") || n.startsWith("shree") || n.startsWith("sri") -> "The rising sign carried forward by how far the Moon has travelled through its nakshatra."
        else -> "A clock-lagna counted from the Sun at sunrise."
    }
}

private fun moonBlurb(k: MoonKind): String = when (k) {
    MoonKind.Gandanta -> "The Moon in the last degrees of a water sign or the first of a fire sign: the knot between them."
    MoonKind.Panchaka -> "The Moon in the last five nakshatras, Dhanishta to Revati: begin nothing that should not come back."
    MoonKind.Bhadra -> "Vishti karana: the half-tithi to avoid for beginnings."
    MoonKind.Yoga -> "The inauspicious yogas of Sun and Moon: Vyatipata, Vaidhriti and the rest."
    MoonKind.Tara -> "The Moon's nakshatra counted from yours: the good taras in gold, the rest to avoid."
    MoonKind.Chandra -> "The Moon's sign counted from your natal Moon: the good houses in gold."
}

/** Both edges to the second; the end drops its date when it shares the start's. */
private fun spanText(start: ExactInstant, end: ExactInstant, zone: ZoneId): String {
    val a = Instants.toLocal(start, zone)
    val b = Instants.toLocal(end, zone)
    return a.format(DATE_TIME_S) + " – " + (if (a.toLocalDate() == b.toLocalDate()) b.format(HMS) else b.format(DATE_TIME_S))
}

/** Epoch milliseconds of an exact instant, for the ticking clock and reminders. */
private fun millisOf(e: ExactInstant): Long = Instants.toLocal(e, ZoneOffset.UTC).toInstant().toEpochMilli()
