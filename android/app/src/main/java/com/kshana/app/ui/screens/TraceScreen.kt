@file:OptIn(ExperimentalLayoutApi::class)

package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.HorizontalDivider
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
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.Finest
import uniffi.kshana_ffi.DashaSensitivityRecord
import uniffi.kshana_ffi.SnapshotRecord
import uniffi.kshana_ffi.dashaSensitivity
import uniffi.kshana_ffi.TransitionRecord
import uniffi.kshana_ffi.dashaLevelNames
import java.time.DateTimeException
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter
import com.kshana.app.ui.sky.KshanaFonts
import com.kshana.app.ui.sky.BellHost
import com.kshana.app.ui.sky.BellInstant

/* ------------------------------------------------------------------------------------
 * The River: the person's life as exact instants. One day at a time, every boundary
 * the engine can solve (dasha levels, the Moon and the planets through their
 * divisions, tithi/karana/yoga, horas, sunrise and sunset), each to the microsecond.
 * ---------------------------------------------------------------------------------- */

private val DAY_TITLE get() = com.kshana.app.ui.screens.datePattern("EEE d MMM uuuu")
private val HMS = DateTimeFormatter.ofPattern("HH:mm:ss")
private val WEEKDAY_HMS get() = com.kshana.app.ui.screens.datePattern("EEE HH:mm:ss")

/** Transitions closer than this share one row of the river. */
private const val ROW_WINDOW_NS = 1_000_000L

/** How far ahead [NextChanges] looks. */
private const val NEXT_WINDOW_SECONDS = 36L * 3600L

/**
 * What a day is solved for. `dashaDepth` 0 switches the dasha off; `moon` NONE switches
 * the Moon off; planets are traced to the sign when on.
 */
private data class TraceFilter(
    val dashaDepth: Int = 5,
    val moon: Finest = Finest.SUB_SUB,
    val planets: Boolean = true,
    val panchanga: Boolean = true,
    val horas: Boolean = true,
    val lagna: Finest = Finest.NAKSHATRA,
)

/** A solved day in the cache: the local date and the filters it was solved with. */
private data class DayKey(val date: LocalDate, val filter: TraceFilter)

/** One row of the river: every transition within a millisecond of the first. */
private class RiverRow(
    val at: ExactInstant,
    val epochMillis: Long,
    val local: ZonedDateTime,
    val items: List<TransitionRecord>,
)

@Composable
fun TraceScreen(profile: BirthProfile, here: Place, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val zone = remember(here, profile) { zoneFor(here, profile) }

    // The clock: ticks every 100 ms. Only the composables that read it recompose at that rate.
    val nowState = remember { mutableStateOf(System.currentTimeMillis()) }
    LaunchedEffect(Unit) {
        while (true) {
            delay(100)
            nowState.value = System.currentTimeMillis()
        }
    }
    val today by remember(zone) { derivedStateOf { Instant.ofEpochMilli(nowState.value).atZone(zone).toLocalDate() } }

    var date by remember(zone) { mutableStateOf(LocalDate.now(zone)) }
    var filter by remember { mutableStateOf(TraceFilter()) }
    var lastDepth by remember { mutableStateOf(5) }
    var moreLevels by remember { mutableStateOf(false) }

    var natalMoon by remember(profile, settings) { mutableStateOf<Double?>(null) }
    var sensitivity by remember(profile, settings, com.kshana.app.data.Lang.code) { mutableStateOf<DashaSensitivityRecord?>(null) }
    var levels by remember { mutableStateOf<List<String>>(emptyList()) }
    var snapshot by remember(profile, here, settings, com.kshana.app.data.Lang.code) { mutableStateOf<SnapshotRecord?>(null) }
    val cache = remember(profile, here, settings, com.kshana.app.data.Lang.code) { mutableStateMapOf<DayKey, List<TransitionRecord>>() }
    var error by remember { mutableStateOf<String?>(null) }

    // The natal Moon is read once from the birth chart; it seeds every dasha computation.
    LaunchedEffect(profile, settings, com.kshana.app.data.Lang.code) {
        try {
            val engine = EngineProvider.get(context)
            val chart = withContext(Dispatchers.Default) {
                engine.chart(profile.birthInstant(), profile.latitude, profile.longitude, 0.0, settings.houseSystem, settings.ayanamsa, false)
            }
            natalMoon = chart.panchanga.moonSidereal
            val moonSpeed = chart.bodies.firstOrNull { it.name == "Moon" }?.speed ?: 13.2
            sensitivity = dashaSensitivity(chart.panchanga.moonSidereal, moonSpeed, profile.uncertaintySeconds().toDouble(), com.kshana.app.data.Lang.code)
            levels = dashaLevelNames(com.kshana.app.data.Lang.code)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    // Solve the shown day for the chosen filters, once per (day, filters).
    val key = DayKey(date, filter)
    LaunchedEffect(natalMoon, here, settings, key, com.kshana.app.data.Lang.code) {
        val moon = natalMoon ?: return@LaunchedEffect
        if (cache.containsKey(key)) return@LaunchedEffect
        error = null
        try {
            val engine = EngineProvider.get(context)
            val from = dayStart(key.date, zone)
            val to = dayStart(key.date.plusDays(1), zone)
            val list = withContext(Dispatchers.Default) {
                engine.trace(
                    profile.birthInstant(), moon, here.latitude, here.longitude, settings.ayanamsa,
                    from, to,
                    key.filter.dashaDepth.toUByte(),
                    key.filter.moon,
                    if (key.filter.planets) Finest.SIGN else Finest.NONE,
                    key.filter.panchanga,
                    key.filter.horas,
                    key.filter.lagna,
                    com.kshana.app.data.Lang.code,
                )
            }
            cache[key] = list
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    val records = cache[key]
    val rows = remember(records, zone) { records?.let { groupRows(it, zone) } ?: emptyList() }
    // How many rows lie behind the present; changes only when a boundary passes.
    val pastCount by remember(rows) {
        derivedStateOf {
            firstAfter(rows, nowState.value)
        }
    }
    val isToday = date == today
    val nextRow = rows.getOrNull(pastCount)
    val solving = records == null && error == null

    // What is running now: read at load, and again each time a boundary of today passes.
    val boundary = if (isToday) pastCount else -1
    LaunchedEffect(profile, here, settings, natalMoon, boundary, com.kshana.app.data.Lang.code) {
        val moon = natalMoon ?: return@LaunchedEffect
        try {
            val engine = EngineProvider.get(context)
            snapshot = withContext(Dispatchers.Default) {
                engine.snapshot(profile.birthInstant(), moon, here.latitude, here.longitude, settings.ayanamsa, Instants.now(), 9u, com.kshana.app.data.Lang.code)
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    LazyColumn(
        Modifier.fillMaxSize(),
        contentPadding = PaddingValues(start = 24.dp, end = 24.dp, top = 48.dp, bottom = 32.dp),
    ) {
        item(key = "header") {
            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                    Text("THE RIVER · ${here.name.uppercase()}", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                    Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
                }
                Text("The river", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
                Text(
                    "Your life as exact instants: every boundary this day crosses, solved to the microsecond.",
                    style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
                )
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                    QuietButton("‹", onClick = { date = date.minusDays(1) })
                    Text(
                        date.format(DAY_TITLE),
                        style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text,
                        textAlign = TextAlign.Center,
                        modifier = Modifier.weight(1f),
                    )
                    QuietButton("›", onClick = { date = date.plusDays(1) })
                    Chip("Today", selected = isToday, onClick = { date = today })
                }
            }
        }

        item(key = "running") {
            Spacer(Modifier.height(14.dp))
            RunningNowCard(snapshot, levels, sensitivity, zone, today, nowState, nextRow, solving, moreLevels) { moreLevels = it }
        }

        item(key = "filters") {
            SectionLabel("Trace")
            FilterRow(filter, lastDepth, onDepth = { lastDepth = it }) { filter = it }
        }

        item(key = "status") {
            Column(Modifier.padding(top = 12.dp)) {
                ErrorLine(error)
                if (solving) {
                    Text("Solving the day…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                } else if (records != null && rows.isEmpty()) {
                    Text("Nothing crosses a boundary on this day with these filters.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                }
            }
        }

        itemsIndexed(rows, key = { _, r -> "${r.at.taiSeconds}.${r.at.nanos}" }) { i, r ->
            if (isToday && i == pastCount) NowDivider()
            RiverRowView(r, past = i < pastCount)
        }
        if (isToday && rows.isNotEmpty() && pastCount == rows.size) {
            item(key = "now-end") { NowDivider() }
        }

        item(key = "footer") {
            HorizontalDivider(color = KshanaColors.LineSoft)
            Column(Modifier.padding(top = 12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                if (records != null) {
                    Text(
                        "${records.size} transitions · each solved to 1 µs · dasha boundaries exact",
                        style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
                    )
                }
                Text("${zone.id} · observed at ${here.name}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            }
        }
    }
}

/* ---------------------------------- Running now ---------------------------------- */

/** The dasha chain, the Moon's lords and the hora as they stand this instant, then the countdown. */
@Composable
private fun RunningNowCard(
    snap: SnapshotRecord?,
    levels: List<String>,
    sensitivity: DashaSensitivityRecord?,
    zone: ZoneId,
    today: LocalDate,
    nowState: State<Long>,
    nextRow: RiverRow?,
    solving: Boolean,
    more: Boolean,
    onMore: (Boolean) -> Unit,
) {
    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("RUNNING NOW", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        if (snap == null) {
            Text("Reading the running periods…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        } else {
            val lords = snap.dashaLords
            val shown = if (more) lords.size else minOf(5, lords.size)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                for (i in 0 until shown) {
                    if (i > 0) {
                        Text("›", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(top = 6.dp))
                    }
                    val meaningful = sensitivity?.let { i < it.meaningfulDepth.toInt() } ?: true
                    DashaChip(levelShort(levels, i), lords[i], snap.dashaEnds.getOrNull(i), zone, today, meaningful)
                }
                if (lords.size > 5) {
                    Text(
                        if (more) "LESS" else "MORE",
                        style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                        modifier = Modifier.clickable { onMore(!more) }.padding(horizontal = 6.dp, vertical = 10.dp),
                    )
                }
            }
            if (sensitivity != null) {
                Text(sensitivity.words, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, lineHeight = 15.sp)
            }
            val rising = snap.grahas.firstOrNull { it.name == "Lagna" }
            if (rising != null) {
                LabelledMono(
                    "RISING",
                    "${signName(rising.sidereal)} ${degMin(rising.sidereal)} · star ${rising.starLord} · sub ${rising.subLord} · sub-sub ${rising.subsubLord}",
                    KshanaColors.Gold,
                )
            }
            val moon = snap.grahas.firstOrNull { it.name == "Moon" }
            if (moon != null) {
                LabelledMono(
                    "MOON",
                    "${signName(moon.sidereal)} ${degMin(moon.sidereal)} · star ${moon.starLord} · sub ${moon.subLord} · sub-sub ${moon.subsubLord}",
                    KshanaColors.Sky,
                )
            }
            val hora = snap.horaLord
            LabelledMono(
                "HORA",
                if (hora == null) "the Sun does not rise here today"
                else hora + (snap.horaEnd?.let { " · until ${endText(it, zone, today)}" } ?: ""),
                KshanaColors.TextSoft,
            )
        }
        HorizontalDivider(color = KshanaColors.LineSoft)
        NextChange(nowState, nextRow, solving)
    }
}

/** One link of the chain: "Antar Venus" over its exact end. */
@Composable
private fun DashaChip(level: String, lord: String, end: ExactInstant?, zone: ZoneId, today: LocalDate, meaningful: Boolean = true) {
    val shape = RoundedCornerShape(6.dp)
    Column(
        Modifier.alpha(if (meaningful) 1f else 0.45f).clip(shape).background(KshanaColors.Night).border(1.dp, KshanaColors.Line, shape).padding(horizontal = 8.dp, vertical = 4.dp),
    ) {
        Text("$level $lord", fontFamily = KshanaFonts.sans, fontSize = 12.sp, color = KshanaColors.Text)
        Text(
            if (end != null) "to ${endText(end, zone, today)}" else "—",
            fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted,
        )
    }
}

@Composable
private fun LabelledMono(label: String, value: String, colour: Color) {
    Column {
        Text(label, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        Text(value, fontFamily = KshanaFonts.mono, fontSize = 11.sp, lineHeight = 16.sp, color = colour)
    }
}

/** "next change in mm:ss.mmm" and what it is; the digits tick with the clock. */
@Composable
private fun NextChange(nowState: State<Long>, next: RiverRow?, solving: Boolean) {
    if (next == null) {
        Text(
            if (solving) "Solving the day…" else "No more changes in this day.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        return
    }
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text("next change in ", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        CountdownText(nowState, next.epochMillis, 14)
    }
    val extra = next.items.size - 1
    Text(
        next.items.first().words + if (extra > 0) " · and $extra more at the same instant" else "",
        style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
        maxLines = 1, overflow = TextOverflow.Ellipsis,
    )
}

/** The only composable that recomposes every tick. */
@Composable
private fun CountdownText(nowState: State<Long>, targetMillis: Long, sizeSp: Int) {
    Text(countdownText(targetMillis - nowState.value), fontFamily = KshanaFonts.mono, fontSize = sizeSp.sp, color = KshanaColors.Gold)
}

/* ------------------------------------ Filters ------------------------------------ */

@Composable
private fun FilterRow(filter: TraceFilter, lastDepth: Int, onDepth: (Int) -> Unit, onChange: (TraceFilter) -> Unit) {
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Chip("Dasha", selected = filter.dashaDepth > 0, onClick = {
            onChange(filter.copy(dashaDepth = if (filter.dashaDepth > 0) 0 else lastDepth))
        })
        for (depth in listOf(5, 7, 9)) {
            Chip("$depth", selected = filter.dashaDepth == depth, onClick = {
                onDepth(depth)
                onChange(filter.copy(dashaDepth = depth))
            })
        }
        Chip("Moon to sub-sub", selected = filter.moon == Finest.SUB_SUB, onClick = {
            onChange(filter.copy(moon = if (filter.moon == Finest.SUB_SUB) Finest.NONE else Finest.SUB_SUB))
        })
        Chip("Moon to sub", selected = filter.moon == Finest.SUB, onClick = {
            onChange(filter.copy(moon = if (filter.moon == Finest.SUB) Finest.NONE else Finest.SUB))
        })
        Chip("Planets", selected = filter.planets, onClick = { onChange(filter.copy(planets = !filter.planets)) })
        Chip("Panchanga", selected = filter.panchanga, onClick = { onChange(filter.copy(panchanga = !filter.panchanga)) })
        Chip("Horas", selected = filter.horas, onClick = { onChange(filter.copy(horas = !filter.horas)) })
        Chip("Rising sign to nakshatra", selected = filter.lagna == Finest.NAKSHATRA, onClick = {
            onChange(filter.copy(lagna = if (filter.lagna == Finest.NAKSHATRA) Finest.NONE else Finest.NAKSHATRA))
        })
        Chip("Rising sign to sub", selected = filter.lagna == Finest.SUB, onClick = {
            onChange(filter.copy(lagna = if (filter.lagna == Finest.SUB) Finest.NONE else Finest.SUB))
        })
        Chip("Rising sign to sub-sub", selected = filter.lagna == Finest.SUB_SUB, onClick = {
            onChange(filter.copy(lagna = if (filter.lagna == Finest.SUB_SUB) Finest.NONE else Finest.SUB_SUB))
        })
    }
}

/* ------------------------------------ The river ----------------------------------- */

/** Local time to the microsecond on the left, the words on the right, one line per transition. */
@Composable
private fun RiverRowView(row: RiverRow, past: Boolean) {
    Column(Modifier.fillMaxWidth().alpha(if (past) 0.7f else 1f)) {
        HorizontalDivider(color = KshanaColors.LineSoft)
        Row(Modifier.fillMaxWidth().padding(vertical = 8.dp), verticalAlignment = Alignment.Top) {
            Column(Modifier.padding(end = 12.dp, top = 2.dp)) {
                Row {
                    Text(row.local.format(HMS), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Text)
                    Text(microSuffix(row.at), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Muted)
                }
                Text(physicalBar(row), fontFamily = KshanaFonts.mono, fontSize = 9.sp, color = KshanaColors.Muted)
            }
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                row.items.forEachIndexed { i, t ->
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            t.words,
                            style = MaterialTheme.typography.bodyMedium,
                            color = if (i == 0) KshanaColors.TextSoft else KshanaColors.Muted,
                            modifier = Modifier.weight(1f).padding(end = 8.dp),
                        )
                        KindTag(t)
                    }
                }
            }
        }
    }
}

/**
 * "sky ±0.3 s" / "exact": the largest physical error bar in the row, from the engine's
 * ephemeris and model limits; a dasha boundary is exact arithmetic.
 */
private fun physicalBar(row: RiverRow): String {
    val worst = row.items.maxOf { it.physicalNs }
    return when {
        worst <= 0L -> "exact"
        worst < 1_000_000_000L -> "sky ±${worst / 1_000_000} ms"
        worst < 120_000_000_000L -> "sky ±${worst / 1_000_000_000} s"
        else -> "sky ±${worst / 60_000_000_000} min"
    }
}

/** A thin gold line at the present instant. */
@Composable
private fun NowDivider() {
    Row(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
        HorizontalDivider(Modifier.weight(1f), color = KshanaColors.Gold)
        Text("NOW", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.padding(horizontal = 8.dp))
        HorizontalDivider(Modifier.weight(1f), color = KshanaColors.Gold)
    }
}

/** The kind of a transition as a small tag: dasha gold, Moon sky, planets text, panchanga soft, hora muted, Sun events gold outline. */
@Composable
private fun KindTag(t: TransitionRecord) {
    val colour = tagColour(t)
    val outlined = t.kind == "sunrise" || t.kind == "sunset"
    val shape = RoundedCornerShape(4.dp)
    Box(
        Modifier
            .clip(shape)
            .then(if (outlined) Modifier.border(1.dp, colour, shape) else Modifier.background(KshanaColors.LineSoft))
            .padding(horizontal = 6.dp, vertical = 2.dp),
    ) {
        Text(tagLabel(t).uppercase(), style = MaterialTheme.typography.labelSmall, color = colour)
    }
}

/* ----------------------------------- Next changes --------------------------------- */

/**
 * The next [count] instants from now, with live countdowns, for other screens. Solves
 * now to now + 36 h at dasha depth 5, Moon to the sub, planets to the sign, with panchanga
 * and horas; re-solves when the window runs low.
 */
@Composable
fun NextChanges(profile: BirthProfile, here: Place, settings: EngineSettings, count: Int = 5) {
    val context = LocalContext.current
    val zone = remember(here, profile) { zoneFor(here, profile) }

    val nowState = remember { mutableStateOf(System.currentTimeMillis()) }
    LaunchedEffect(Unit) {
        while (true) {
            delay(100)
            nowState.value = System.currentTimeMillis()
        }
    }

    var rows by remember(profile, here, settings, com.kshana.app.data.Lang.code) { mutableStateOf<List<RiverRow>>(emptyList()) }
    var loadedUntil by remember(profile, here, settings, com.kshana.app.data.Lang.code) { mutableStateOf(0L) }
    var loading by remember { mutableStateOf(true) }
    var error by remember { mutableStateOf<String?>(null) }

    val upcoming by remember(rows, count) {
        derivedStateOf {
            val i = firstAfter(rows, nowState.value)
            rows.subList(i, minOf(rows.size, i + count))
        }
    }
    // Re-solve when fewer than `count` remain and the solved window ends within twelve hours.
    val stale by remember(rows, count) {
        derivedStateOf {
            val now = nowState.value
            loadedUntil == 0L || (rows.size - firstAfter(rows, now) < count && loadedUntil - now < 12L * 3_600_000L)
        }
    }

    LaunchedEffect(profile, here, settings, stale, com.kshana.app.data.Lang.code) {
        if (!stale) return@LaunchedEffect
        loading = true
        try {
            val engine = EngineProvider.get(context)
            val fromInstant = Instant.now()
            val toInstant = fromInstant.plusSeconds(NEXT_WINDOW_SECONDS)
            val list = withContext(Dispatchers.Default) {
                val chart = engine.chart(profile.birthInstant(), profile.latitude, profile.longitude, 0.0, settings.houseSystem, settings.ayanamsa, false)
                engine.trace(
                    profile.birthInstant(), chart.panchanga.moonSidereal, here.latitude, here.longitude, settings.ayanamsa,
                    Instants.fromInstant(fromInstant), Instants.fromInstant(toInstant),
                    5u, Finest.SUB, Finest.SIGN, true, true, Finest.NAKSHATRA, com.kshana.app.data.Lang.code,
                )
            }
            rows = groupRows(list, zone)
            loadedUntil = toInstant.toEpochMilli()
            error = null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: e.toString()
            loadedUntil = System.currentTimeMillis() + 36L * 3_600_000L // do not retry every tick
        }
        loading = false
    }

    Column(Modifier.fillMaxWidth()) {
        ErrorLine(error)
        BellHost(instants = upcoming.flatMap { r -> r.items.map { BellInstant(r.epochMillis, bellKind(it), it.words) } })
        if (upcoming.isEmpty()) {
            Text(
                if (loading) "Solving the next changes…" else "Nothing changes in the next 36 hours.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
        }
        val todayHere = Instant.ofEpochMilli(nowState.value).atZone(zone).toLocalDate()
        for (r in upcoming) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            Row(Modifier.fillMaxWidth().padding(vertical = 8.dp), verticalAlignment = Alignment.Top) {
                Column(Modifier.weight(1f).padding(end = 10.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    r.items.forEachIndexed { i, t ->
                        Text(
                            t.words,
                            style = MaterialTheme.typography.bodyMedium,
                            color = if (i == 0) KshanaColors.TextSoft else KshanaColors.Muted,
                            maxLines = 2, overflow = TextOverflow.Ellipsis,
                        )
                    }
                    Text(
                        (if (r.local.toLocalDate() == todayHere) "today " + r.local.format(HMS) else r.local.format(WEEKDAY_HMS)) + microSuffix(r.at),
                        fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted,
                    )
                }
                Column(horizontalAlignment = Alignment.End, verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    CountdownText(nowState, r.epochMillis, 13)
                    KindTag(r.items.first())
                }
            }
        }
    }
}

/* ------------------------------------- Helpers ------------------------------------ */

/** Where the person is now, falling back to the birth zone and then the device. */
private fun zoneFor(here: Place, profile: BirthProfile): ZoneId =
    try { com.kshana.app.data.CityClocks.of(here.zoneId) } catch (e: DateTimeException) { profile.zone() }

/** Local midnight of a civil date as an exact instant. */
private fun dayStart(date: LocalDate, zone: ZoneId): ExactInstant =
    Instants.fromInstant(date.atStartOfDay(zone).toInstant())

/** Index of the first row after `nowMillis` in a time-sorted list (its size when none is). */
private fun firstAfter(rows: List<RiverRow>, nowMillis: Long): Int {
    var lo = 0
    var hi = rows.size
    while (lo < hi) {
        val mid = (lo + hi) ushr 1
        if (rows[mid].epochMillis > nowMillis) hi = mid else lo = mid + 1
    }
    return lo
}

private fun nanosBetween(a: ExactInstant, b: ExactInstant): Long =
    (b.taiSeconds - a.taiSeconds) * 1_000_000_000L + (b.nanos.toLong() - a.nanos.toLong())

/** ".SSSSSS": the microseconds of the instant, straight from its nanoseconds. */
private fun microSuffix(e: ExactInstant): String =
    "." + (e.nanos.toLong() / 1_000).toString().padStart(6, '0')

/** Today: the time to the microsecond; another day: date and time to the second. */
private fun endText(e: ExactInstant, zone: ZoneId, today: LocalDate): String {
    val l = Instants.toLocal(e, zone)
    return if (l.toLocalDate() == today) l.format(HMS) + microSuffix(e) else l.format(DATE_TIME_S)
}

/** "Mahadasha" → "Maha"; levels without a proper name keep the engine's label. */
private fun levelShort(levels: List<String>, i: Int): String =
    levels.getOrNull(i)?.removeSuffix("dasha") ?: "Level ${i + 1}"

/** "mm:ss.mmm", or "h:mm:ss.mmm" beyond an hour; never negative. */
private fun countdownText(remainingMillis: Long): String {
    val ms = remainingMillis.coerceAtLeast(0L)
    val h = ms / 3_600_000L
    val m = (ms / 60_000L) % 60L
    val s = (ms / 1_000L) % 60L
    val milli = ms % 1_000L
    return if (h > 0) "%d:%02d:%02d.%03d".format(h, m, s, milli) else "%02d:%02d.%03d".format(m, s, milli)
}

private val DIVISION_ORDER = listOf("sign", "nakshatra", "pada", "sub", "sub-sub")

/** Order within one instant: dasha by level, Sun events, the Moon coarse to fine, the planets, panchanga, horas. */
private fun rank(t: TransitionRecord): Int = when (t.kind) {
    "dasha" -> t.level.toInt()
    "sunrise", "sunset" -> 10
    "enter" -> 20 + (if (t.body == "Moon") 0 else 10) + DIVISION_ORDER.indexOf(t.division).coerceAtLeast(0)
    "tithi" -> 40
    "karana" -> 41
    "yoga" -> 42
    "hora" -> 50
    else -> 60
}

/** Sort by instant and merge everything within a millisecond into one row. */
private fun groupRows(records: List<TransitionRecord>, zone: ZoneId): List<RiverRow> {
    val sorted = records.sortedWith(compareBy({ it.at.taiSeconds }, { it.at.nanos }))
    val rows = mutableListOf<RiverRow>()
    var group = mutableListOf<TransitionRecord>()
    fun flush() {
        if (group.isEmpty()) return
        val items = group.sortedBy { rank(it) }
        val at = items.first().at
        val local = Instants.toLocal(at, zone)
        rows += RiverRow(at, local.toInstant().toEpochMilli(), local, items)
        group = mutableListOf()
    }
    for (r in sorted) {
        val head = group.firstOrNull()
        if (head != null && nanosBetween(head.at, r.at) >= ROW_WINDOW_NS) flush()
        group += r
    }
    flush()
    return rows
}

private fun tagLabel(t: TransitionRecord): String = when (t.kind) {
    "enter" -> if (t.body == "Lagna") "rising" else t.body
    else -> t.kind
}

private fun tagColour(t: TransitionRecord): Color = when (t.kind) {
    "dasha" -> KshanaColors.Gold
    "enter" -> when (t.body) {
        "Moon" -> KshanaColors.Sky
        "Lagna" -> KshanaColors.Gold
        else -> KshanaColors.Text
    }
    "tithi", "karana", "yoga" -> KshanaColors.TextSoft
    "hora" -> KshanaColors.Muted
    "sunrise", "sunset" -> KshanaColors.Gold
    else -> KshanaColors.Muted
}

/** The haptic and bell phrase for a transition: dasha depth, the Moon, the rising sign, the day's edges. */
private fun bellKind(t: TransitionRecord): String = when (t.kind) {
    "dasha" -> if (t.level.toInt() <= 2) "dasha" else "dasha.minor"
    "sunrise" -> "sunrise"
    "sunset" -> "sunset"
    "hora" -> "hora"
    "enter" -> when {
        t.body == "Lagna" -> "sub"
        t.division == "sub" || t.division == "sub-sub" -> "sub"
        else -> "moon"
    }
    else -> "moon"
}
