package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.CandidateRecord
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.LedgerEventRecord
import uniffi.kshana_ffi.LifeEventRecord
import uniffi.kshana_ffi.PosteriorRecord
import kotlin.math.ceil
import java.time.DateTimeException
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter
import com.kshana.app.ui.sky.KshanaFonts

/** The eight life areas the engine scores, by their engine names. */
private val AREAS = listOf("Career", "Marriage", "Children", "Wealth", "Health", "Property", "Education", "Foreign")

private const val EVENTS_KEY = "rectify:events"
private const val TOP = 5

/** Half-window used when the profile claims an exact time: ±15 min. */
private const val DEFAULT_HALF_SECONDS = 900L

private val CLOCK = DateTimeFormatter.ofPattern("HH:mm")

/** One dated event as the user states it: local civil time in the profile's zone. */
private data class LifeEvent(val area: String, val year: Int, val month: Int, val day: Int, val hour: Int, val minute: Int, val label: String) {
    fun local(zone: ZoneId): ZonedDateTime = ZonedDateTime.of(LocalDateTime.of(year, month, day, hour, minute, 0), zone)

    fun instant(zone: ZoneId): ExactInstant = Instants.fromInstant(local(zone).toInstant())

    /** `area|year|month|day|hour|minute|label`; the label is stripped of the delimiters first. */
    fun serialize(): String = listOf(area, year, month, day, hour, minute, label.replace('|', ' ').replace('\n', ' ')).joinToString("|")

    companion object {
        fun parse(s: String): LifeEvent? {
            val f = s.split('|')
            if (f.size < 7) return null
            return LifeEvent(
                area = f[0],
                year = f[1].toIntOrNull() ?: return null,
                month = f[2].toIntOrNull() ?: return null,
                day = f[3].toIntOrNull() ?: return null,
                hour = f[4].toIntOrNull() ?: return null,
                minute = f[5].toIntOrNull() ?: return null,
                label = f.drop(6).joinToString("|"),
            )
        }
    }
}

private fun loadEvents(s: String?): List<LifeEvent> =
    s.orEmpty().split('\n').filter { it.isNotBlank() }.mapNotNull { LifeEvent.parse(it) }

private fun storeEvents(events: List<LifeEvent>): String = events.joinToString("\n") { it.serialize() }

/** Step for the scan: a minute up to an hour, two minutes up to three hours, five beyond. */
private fun stepFor(halfSeconds: Long): Long = when {
    halfSeconds <= 3600L -> 60L
    halfSeconds <= 10800L -> 120L
    else -> 300L
}

/** "±15 min" / "±1 h" for a half-window in seconds. */
private fun windowText(halfSeconds: Long): String =
    if (halfSeconds < 3600L) "±${halfSeconds / 60} min" else "±${halfSeconds / 3600} h"

/** The most candidate instants the posterior scans; the engine widens the step past it anyway. */
private const val MAX_POSTERIOR_CANDIDATES = 400L

/** The finest step that keeps `2·half/step + 1` candidates at or under the cap, at least a second. */
private fun posteriorStep(halfSeconds: Long): Long =
    ceil(2.0 * halfSeconds / (MAX_POSTERIOR_CANDIDATES - 1).toDouble()).toLong().coerceAtLeast(1L)

/** "one piece" / "2 pieces" for a highest-density set's run count. */
private fun pieces(runs: UInt): String = if (runs <= 1u) "one piece" else "$runs pieces"

/** The events of this screen as the ledger's record: each a milestone in its area, full weight, the label as the note. */
private fun LifeEvent.ledgerRecord(zone: ZoneId): LedgerEventRecord =
    LedgerEventRecord(instant(zone), area, "Milestone", 1.0, label)

/**
 * Birth-time rectification: dated life events in, every candidate minute in the
 * uncertainty window cast and scored, the best five offered back as a new birth time.
 */
@Composable
fun RectifyScreen(profile: BirthProfile, settings: EngineSettings, onSaved: (BirthProfile) -> Unit, onBack: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val zone = remember(profile) { profile.zone() }
    val halfSeconds = remember(profile) { if (profile.uncertaintyLevel == 0) DEFAULT_HALF_SECONDS else profile.uncertaintySeconds() }
    val stepSeconds = remember(halfSeconds) { stepFor(halfSeconds) }

    var events by remember { mutableStateOf(loadEvents(ProfileStore.prefs(context).getString(EVENTS_KEY, null))) }
    var area by remember { mutableStateOf(AREAS[0]) }
    var year by remember { mutableStateOf("") }
    var month by remember { mutableStateOf("") }
    var day by remember { mutableStateOf("") }
    var hour by remember { mutableStateOf("") }
    var minute by remember { mutableStateOf("") }
    var label by remember { mutableStateOf("") }
    var problem by remember { mutableStateOf<String?>(null) }

    var running by remember { mutableStateOf(false) }
    var candidates by remember { mutableStateOf<List<CandidateRecord>>(emptyList()) }
    var status by remember { mutableStateOf<String?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var saved by remember { mutableStateOf<String?>(null) }

    var posterior by remember { mutableStateOf<PosteriorRecord?>(null) }
    var posteriorRunning by remember { mutableStateOf(false) }
    var posteriorStatus by remember { mutableStateOf<String?>(null) }
    var posteriorError by remember { mutableStateOf<String?>(null) }

    fun persist(list: List<LifeEvent>) {
        events = list
        ProfileStore.prefs(context).edit().putString(EVENTS_KEY, storeEvents(list)).apply()
        posterior = null
        posteriorStatus = null
    }

    fun add() {
        val y = year.toIntOrNull(); val mo = month.toIntOrNull(); val d = day.toIntOrNull()
        val h = hour.toIntOrNull() ?: 12; val mi = minute.toIntOrNull() ?: 0
        if (y == null || mo == null || d == null) { problem = "The event needs a year, a month and a day."; return }
        try { LocalDateTime.of(y, mo, d, h, mi, 0) } catch (e: DateTimeException) { problem = "That date and time do not exist on the calendar."; return }
        val text = label.trim().ifEmpty { area }
        problem = null
        persist(events + LifeEvent(area, y, mo, d, h, mi, text))
        year = ""; month = ""; day = ""; hour = ""; minute = ""; label = ""
        candidates = emptyList()
        status = null
    }

    fun run() {
        if (events.isEmpty() || running) return
        running = true
        error = null
        saved = null
        candidates = emptyList()
        val n = 2L * halfSeconds / stepSeconds + 1L
        status = "casting $n candidate charts…"
        scope.launch {
            try {
                val engine = EngineProvider.get(context)
                val center = profile.birthInstant()
                val records = events.map { LifeEventRecord(it.area, it.instant(zone), it.label) }
                val found = withContext(Dispatchers.Default) {
                    engine.rectify(center, halfSeconds.toULong(), stepSeconds.toULong(), profile.latitude, profile.longitude, settings.ayanamsa, records)
                }
                candidates = found
                status = if (found.isEmpty()) "No candidate could be scored in this window." else "${found.size} candidates · best first · ${events.size} events"
            } catch (e: Exception) {
                error = e.message ?: e.toString()
                status = null
            }
            running = false
        }
    }

    fun drawPosterior() {
        if (events.isEmpty() || posteriorRunning) return
        posteriorRunning = true
        posteriorError = null
        saved = null
        val step = posteriorStep(halfSeconds)
        val n = 2L * halfSeconds / step + 1L
        posteriorStatus = "casting $n candidate charts in steps of $step s…"
        scope.launch {
            try {
                val engine = EngineProvider.get(context)
                val records = events.map { it.ledgerRecord(zone) }
                val found = withContext(Dispatchers.Default) {
                    engine.birthPosterior(profile.birthInstant(), profile.latitude, profile.longitude, settings.ayanamsa, halfSeconds, step, records)
                }
                posterior = found
                posteriorStatus = "${found.instants.size} candidates · step %.0f s · ${found.events} events".format(found.stepSeconds)
            } catch (e: Exception) {
                posteriorError = e.message ?: e.toString()
                posteriorStatus = null
            }
            posteriorRunning = false
        }
    }

    fun useInstant(birth: ExactInstant) {
        val local = Instants.toLocal(birth, zone)
        val p = profile.copy(
            year = local.year, month = local.monthValue, day = local.dayOfMonth,
            hour = local.hour, minute = local.minute, second = local.second,
            uncertaintyLevel = 1,
        )
        ProfileStore.save(context, p)
        saved = "Saved ${local.format(TIME_ONLY)} as the birth time, ±1 min."
        onSaved(p)
    }

    fun use(c: CandidateRecord) = useInstant(c.birth)

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("RECTIFICATION", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("Fix the time from your life", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "Add dated events. Every candidate minute in your window is cast and scored by how well its dashas explain them.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        Text(
            "Recorded ${profile.birthLocal().format(CLOCK)}, window ${windowText(halfSeconds)}",
            fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky,
        )

        SectionLabel("Events")
        if (events.isEmpty()) {
            Text("No events yet. A marriage, a first job, a move abroad: anything with a date.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        }
        for ((i, e) in events.withIndex()) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            Row(Modifier.fillMaxWidth().heightIn(min = 44.dp), verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text(e.label, fontFamily = KshanaFonts.serif, fontSize = 17.sp, color = KshanaColors.Text)
                    Text("${e.area} · ${e.local(zone).format(DATE_TIME)}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                }
                Box(
                    Modifier.height(44.dp).clickable { persist(events.filterIndexed { j, _ -> j != i }); candidates = emptyList(); status = null }.padding(horizontal = 12.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    Text("REMOVE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                }
            }
        }
        if (events.isNotEmpty()) HorizontalDivider(color = KshanaColors.LineSoft)

        SectionLabel("Add an event")
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (a in AREAS) Chip(a.uppercase(), selected = area == a, onClick = { area = a })
        }
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            EventNumberField(year, { year = it.filter(Char::isDigit).take(4) }, "Year", Modifier.weight(1.4f))
            EventNumberField(month, { month = it.filter(Char::isDigit).take(2) }, "Month", Modifier.weight(1f))
            EventNumberField(day, { day = it.filter(Char::isDigit).take(2) }, "Day", Modifier.weight(1f))
        }
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            EventNumberField(hour, { hour = it.filter(Char::isDigit).take(2) }, "Hour (optional)", Modifier.weight(1f))
            EventNumberField(minute, { minute = it.filter(Char::isDigit).take(2) }, "Minute", Modifier.weight(1f))
        }
        Text("Without a time the event is placed at 12:00 in ${profile.zoneId}.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        EventField(label, { label = it }, "Label (e.g. Married, First job)", Modifier.fillMaxWidth())
        problem?.let { Text(it, color = KshanaColors.Sky, style = MaterialTheme.typography.bodyMedium) }
        QuietButton("ADD", onClick = { add() }, modifier = Modifier.fillMaxWidth())

        Spacer(Modifier.height(8.dp))
        ErrorLine(error)
        GoldButton(if (running) "CASTING…" else "RUN", enabled = events.isNotEmpty() && !running, onClick = { run() }, modifier = Modifier.fillMaxWidth())
        status?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted) }
        saved?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Gold) }

        if (candidates.isNotEmpty()) {
            SectionLabel("Score across the window")
            ScoreCurve(candidates, profile.birthInstant(), halfSeconds, zone)
            Text(
                "Candidates within 0.05 of each other are not distinguishable by this method; add more events to separate them.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
            SectionLabel("Best candidates")
            for (c in candidates.take(TOP)) CandidateCard(c, zone, onUse = { use(c) })
        }

        if (events.isNotEmpty()) {
            SectionLabel("The posterior")
            Text(
                "A ranking says which minute scored best. A posterior says how much of the belief each minute deserves: every candidate across the window is cast, and each event counts only in proportion to how rarely that chart would explain a random instant.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
            ErrorLine(posteriorError)
            QuietButton(if (posteriorRunning) "CASTING…" else "DRAW THE POSTERIOR", onClick = { drawPosterior() }, modifier = Modifier.fillMaxWidth())
            posteriorStatus?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted) }
            posterior?.let { p ->
                PosteriorStrip(p, profile.birthInstant(), zone)
                Text(
                    "on ${localText(p.map, DATE_ONLY, zone)} · ${zone.id}",
                    fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted,
                )
                MonoRow("most likely (MAP)", localText(p.map, TIME_ONLY, zone), valueColor = KshanaColors.Gold)
                MonoRow("posterior mean", localText(p.mean, TIME_ONLY, zone))
                MonoRow("50% band", "${localText(p.hpd50Start, TIME_ONLY, zone)} – ${localText(p.hpd50End, TIME_ONLY, zone)}")
                MonoRow("90% band", "${localText(p.hpd90Start, TIME_ONLY, zone)} – ${localText(p.hpd90End, TIME_ONLY, zone)}")
                MonoRow("grid", "${p.instants.size} candidates · step %.0f s".format(p.stepSeconds))
                Text(
                    "The 50% band is in ${pieces(p.hpd50Runs)}; the 90% band is in ${pieces(p.hpd90Runs)}." +
                        if (p.hpd50Runs > 1u || p.hpd90Runs > 1u) " A band in pieces is a comb, not one time: the events agree with several minutes, and the bands above are the hulls." else "",
                    style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
                )
                Text(p.words, fontFamily = KshanaFonts.serif, fontSize = 16.sp, lineHeight = 22.sp, color = KshanaColors.Text)
                QuietButton("SET THIS AS BIRTH TIME", onClick = { useInstant(p.map) }, modifier = Modifier.fillMaxWidth())
                Text(
                    "Each event is read as a milestone in its area. Dasha boundaries are recomputed exactly for every candidate; the transit part is read from the recorded time's chart, sampled once per pratyantar cell. The prior is flat across the window.",
                    style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
                )
            }
        }

        Spacer(Modifier.height(12.dp))
        Text(
            "Rectification ranks hypotheses. It cannot recover a time that no event constrains.",
            fontFamily = KshanaFonts.serif, fontStyle = FontStyle.Italic, fontSize = 14.sp, lineHeight = 20.sp, color = KshanaColors.Muted,
        )
    }
}

/**
 * Score against candidate birth time over the whole window: a gold line, the recorded
 * time as a thin sky vertical, the best candidate as a gold dot.
 */
@Composable
private fun ScoreCurve(candidates: List<CandidateRecord>, center: ExactInstant, halfSeconds: Long, zone: ZoneId) {
    val byTime = remember(candidates) { candidates.sortedBy { it.birth.seconds() } }
    val best = candidates.first()
    val start = center.seconds() - halfSeconds
    val span = 2.0 * halfSeconds
    val lo = byTime.minOf { it.score }
    val hi = byTime.maxOf { it.score }
    val range = if (hi - lo > 1e-9) hi - lo else 1.0
    val shape = RoundedCornerShape(8.dp)

    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Canvas(Modifier.fillMaxWidth().height(120.dp)) {
            val w = size.width
            val h = size.height
            val inset = 6f
            fun x(e: ExactInstant): Float = (((e.seconds() - start) / span).coerceIn(0.0, 1.0) * w).toFloat()
            fun y(score: Double): Float = (h - inset - ((score - lo) / range) * (h - 2 * inset)).toFloat()

            drawLine(KshanaColors.LineSoft, Offset(0f, h - inset), Offset(w, h - inset), 1f)
            val cx = x(center)
            drawLine(KshanaColors.Sky, Offset(cx, 0f), Offset(cx, h), 1f)

            if (byTime.size >= 2) {
                val path = Path()
                for ((i, c) in byTime.withIndex()) {
                    val px = x(c.birth)
                    val py = y(c.score)
                    if (i == 0) path.moveTo(px, py) else path.lineTo(px, py)
                }
                drawPath(path, KshanaColors.Gold, style = Stroke(width = 2f))
            }
            drawCircle(KshanaColors.Gold, radius = 5f, center = Offset(x(best.birth), y(best.score)))
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(localText(byTime.first().birth, CLOCK, zone), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
            Text("best ${localText(best.birth, TIME_ONLY, zone)} · %.2f".format(best.score), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Gold)
            Text(localText(byTime.last().birth, CLOCK, zone), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
        }
    }
}

/**
 * The posterior as a density strip: the mass of every candidate as a gold fill over
 * time, the 90% and 50% highest-density hulls as sky bands beneath it, the recorded
 * time as a thin sky vertical and the most likely instant as a gold one.
 */
@Composable
private fun PosteriorStrip(p: PosteriorRecord, recorded: ExactInstant, zone: ZoneId) {
    val n = minOf(p.instants.size, p.masses.size)
    if (n == 0) return
    val first = p.instants.first().seconds()
    val last = p.instants[n - 1].seconds()
    val span = if (last - first > 1e-9) last - first else 1.0
    val peak = p.masses.take(n).maxOrNull()?.takeIf { it > 0.0 } ?: 1.0
    val shape = RoundedCornerShape(8.dp)

    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Canvas(Modifier.fillMaxWidth().height(120.dp)) {
            val w = size.width
            val h = size.height
            val inset = 6f
            val base = h - inset
            fun x(e: ExactInstant): Float = (((e.seconds() - first) / span).coerceIn(0.0, 1.0) * w).toFloat()
            fun y(mass: Double): Float = (base - (mass / peak).coerceIn(0.0, 1.0) * (h - 2 * inset)).toFloat()

            val x90a = x(p.hpd90Start); val x90b = x(p.hpd90End)
            drawRect(KshanaColors.Sky.copy(alpha = 0.10f), topLeft = Offset(x90a, 0f), size = Size((x90b - x90a).coerceAtLeast(1f), h))
            val x50a = x(p.hpd50Start); val x50b = x(p.hpd50End)
            drawRect(KshanaColors.Sky.copy(alpha = 0.18f), topLeft = Offset(x50a, 0f), size = Size((x50b - x50a).coerceAtLeast(1f), h))

            drawLine(KshanaColors.LineSoft, Offset(0f, base), Offset(w, base), 1f)
            val rx = x(recorded)
            drawLine(KshanaColors.Sky, Offset(rx, 0f), Offset(rx, h), 1f)

            val fill = Path()
            fill.moveTo(x(p.instants[0]), base)
            for (i in 0 until n) fill.lineTo(x(p.instants[i]), y(p.masses[i]))
            fill.lineTo(x(p.instants[n - 1]), base)
            fill.close()
            drawPath(fill, KshanaColors.Gold.copy(alpha = 0.45f))
            if (n >= 2) {
                val line = Path()
                for (i in 0 until n) {
                    val px = x(p.instants[i]); val py = y(p.masses[i])
                    if (i == 0) line.moveTo(px, py) else line.lineTo(px, py)
                }
                drawPath(line, KshanaColors.Gold, style = Stroke(width = 1.5f))
            }
            val mx = x(p.map)
            drawLine(KshanaColors.Gold, Offset(mx, 0f), Offset(mx, h), 2f)
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(localText(p.instants.first(), CLOCK, zone), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
            Text("MAP ${localText(p.map, TIME_ONLY, zone)} · bands 50% · 90%", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Gold)
            Text(localText(p.instants[n - 1], CLOCK, zone), fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
        }
    }
}

/** One candidate: the local time, its score, the Ascendant it gives, and how each event scored. */
@Composable
private fun CandidateCard(c: CandidateRecord, zone: ZoneId, onUse: () -> Unit) {
    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Text(localText(c.birth, TIME_ONLY, zone), fontFamily = KshanaFonts.mono, fontSize = 18.sp, color = KshanaColors.Text)
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            ScoreBar(c.score.toFloat(), Modifier.weight(1f))
            Text("%.2f".format(c.score), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Gold)
        }
        Text(
            "Asc ${degMin(c.ascSidereal)} ${signName(c.ascSidereal)} · sub-lord ${c.ascSubLord}",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
        )
        if (c.perEvent.isNotEmpty()) HorizontalDivider(color = KshanaColors.LineSoft)
        for (e in c.perEvent) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text(e.label, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text, modifier = Modifier.weight(1f))
                Text("%.2f".format(e.score), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky)
            }
            for (f in e.factors) Text("· $f", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        }
        Spacer(Modifier.height(2.dp))
        QuietButton("USE THIS TIME", onClick = onUse, modifier = Modifier.fillMaxWidth())
    }
}

/** A thin gold bar: the score as a fraction of the full width. */
@Composable
private fun ScoreBar(score: Float, modifier: Modifier = Modifier) {
    Box(modifier.height(4.dp).clip(RoundedCornerShape(2.dp)).background(KshanaColors.LineSoft)) {
        Box(Modifier.fillMaxWidth(score.coerceIn(0.002f, 1f)).fillMaxHeight().background(KshanaColors.Gold))
    }
}

@Composable
private fun EventField(value: String, onChange: (String) -> Unit, label: String, modifier: Modifier = Modifier) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label, style = MaterialTheme.typography.labelSmall) },
        singleLine = true,
        textStyle = TextStyle(fontFamily = KshanaFonts.sans, fontSize = 15.sp, color = KshanaColors.Text),
        colors = eventFieldColors(),
        modifier = modifier,
    )
}

@Composable
private fun EventNumberField(value: String, onChange: (String) -> Unit, label: String, modifier: Modifier = Modifier) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label, style = MaterialTheme.typography.labelSmall) },
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
        textStyle = TextStyle(fontFamily = KshanaFonts.mono, fontSize = 15.sp, color = KshanaColors.Text),
        colors = eventFieldColors(),
        modifier = modifier,
    )
}

@Composable
private fun eventFieldColors() = OutlinedTextFieldDefaults.colors(
    focusedTextColor = KshanaColors.Text,
    unfocusedTextColor = KshanaColors.Text,
    cursorColor = KshanaColors.Gold,
    focusedBorderColor = KshanaColors.Gold,
    unfocusedBorderColor = KshanaColors.Line,
    focusedLabelColor = KshanaColors.Gold,
    unfocusedLabelColor = KshanaColors.Muted,
    focusedContainerColor = KshanaColors.Surface,
    unfocusedContainerColor = KshanaColors.Surface,
)
