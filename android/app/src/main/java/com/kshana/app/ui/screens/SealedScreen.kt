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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.SealedEntry
import com.kshana.app.data.SealedStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.sky.KshanaFonts
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.MetricsRecord
import uniffi.kshana_ffi.SealedRecord
import uniffi.kshana_ffi.ledgerEventKinds
import uniffi.kshana_ffi.resolveForecast
import uniffi.kshana_ffi.sealedForecastMetrics
import uniffi.kshana_ffi.sealedForecastToJson
import java.time.DateTimeException
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.ZonedDateTime

/** The eight life areas, by their engine names. */
private val SEALED_AREAS = listOf("Career", "Marriage", "Children", "Wealth", "Health", "Property", "Education", "Foreign")

/** The event kinds the engine accepts, used only if the engine cannot be asked. */
private val SEALED_FALLBACK_KINDS = listOf("Began", "Ended", "Gain", "Loss", "Milestone", "Crisis")

/** Resolved forecasts below which a probability is said in words, not as a percentage. */
private const val MIN_FOR_PERCENT = 10

private fun ExactInstant.isBefore(other: ExactInstant): Boolean =
    taiSeconds < other.taiSeconds || (taiSeconds == other.taiSeconds && nanos < other.nanos)

/** The engine's probability in words, for when too few forecasts have resolved to earn a percentage. */
private fun probabilityWords(p: Double): String = when {
    p >= 0.65 -> "the chart calls this likely"
    p >= 0.5 -> "the chart leans toward it"
    p >= 0.35 -> "the chart leans against it"
    else -> "the chart calls this unlikely"
}

/**
 * Sealed forecasts: a claim about a dated window, committed before the window opens,
 * resolved after it closes, scored with nothing else. The ledger starts at sealed 0,
 * resolved 0, and that is the point.
 */
@Composable
fun SealedScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val zone = remember { ZoneId.systemDefault() }
    val now = remember { Instants.now() }
    val kinds = remember { runCatching { ledgerEventKinds() }.getOrNull()?.takeIf { it.isNotEmpty() } ?: SEALED_FALLBACK_KINDS }

    var entries by remember(profile.name) { mutableStateOf(SealedStore.entries(context, profile.name)) }
    val unreadable = remember(entries) { SealedStore.unreadable(context, profile.name) }
    val resolvedCount = remember(entries) { entries.count { it.record.resolved } }
    val metrics = remember(entries) { runCatching { sealedForecastMetrics(entries.map { it.record }) }.getOrNull() }

    var formOpen by remember { mutableStateOf(false) }
    var sealing by remember { mutableStateOf(false) }
    var resolveError by remember { mutableStateOf<String?>(null) }
    var expanded by remember { mutableStateOf<String?>(null) }

    fun seal(area: String, kind: String, start: ZonedDateTime, end: ZonedDateTime, onProblem: (String) -> Unit) {
        if (sealing) return
        sealing = true
        scope.launch {
            try {
                val engine = EngineProvider.get(context)
                val createdAt = Instants.now()
                val record = withContext(Dispatchers.Default) {
                    engine.sealForecast(
                        profile.birthInstant(), profile.latitude, profile.longitude, settings.ayanamsa,
                        createdAt, area, kind, Instants.fromInstant(start.toInstant()), Instants.fromInstant(end.toInstant()),
                    )
                }
                val line = sealedForecastToJson(record)
                entries = SealedStore.add(context, profile.name, line)
                formOpen = false
            } catch (e: Exception) {
                onProblem(e.message ?: e.toString())
            }
            sealing = false
        }
    }

    fun resolve(entry: SealedEntry, happened: Boolean) {
        resolveError = null
        scope.launch {
            try {
                val record = withContext(Dispatchers.Default) { resolveForecast(entry.line, happened, Instants.now()) }
                val line = sealedForecastToJson(record)
                entries = SealedStore.replace(context, profile.name, entry.line, line)
            } catch (e: Exception) {
                resolveError = e.message ?: e.toString()
            }
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("SEALED FORECASTS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("Say it before it happens", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "A forecast is sealed before its window opens, with the probability the chart gives it and the question that will settle it. After the window closes you say what happened, and only those count.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
        )
        Text(
            "sealed ${entries.size} · resolved $resolvedCount · open ${entries.size - resolvedCount}",
            fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky,
        )
        if (unreadable > 0) {
            Text("$unreadable stored ${if (unreadable == 1) "line" else "lines"} could not be read by the engine; kept, not shown.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }

        if (!formOpen) {
            GoldButton("SEAL A FORECAST", onClick = { formOpen = true }, modifier = Modifier.fillMaxWidth())
        } else {
            SealForm(
                kinds = kinds,
                zone = zone,
                sealing = sealing,
                onSeal = { area, kind, start, end, onProblem -> seal(area, kind, start, end, onProblem) },
                onCancel = { formOpen = false },
            )
        }

        SectionLabel("Forecasts")
        ErrorLine(resolveError)
        if (entries.isEmpty()) {
            Text("Nothing sealed yet. Sealed 0, resolved 0: the honest start.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        }
        for (entry in entries) {
            SealedCard(
                entry = entry,
                zone = zone,
                now = now,
                resolvedCount = resolvedCount,
                open = expanded == entry.record.id,
                onToggle = { expanded = if (expanded == entry.record.id) null else entry.record.id },
                onResolve = { happened -> resolve(entry, happened) },
            )
        }

        SectionLabel("Score")
        if (metrics != null) {
            SealedScore(metrics)
        } else {
            Text("No resolved forecasts yet: sealed ${entries.size}, resolved 0.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        }

        Spacer(Modifier.height(12.dp))
        Text(
            "A forecast counts only if it was sealed before its window opened. The id is a hash of everything above; editing it would show.",
            fontFamily = KshanaFonts.serif, fontStyle = FontStyle.Italic, fontSize = 14.sp, lineHeight = 20.sp, color = KshanaColors.Muted,
        )
        Text(
            "The probability is the mean fit over the window: the dasha part exact, the transit part sampled once per pratyantar cell.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )
    }
}

/** One sealed forecast: the claim, its window, the chart's probability, the question, its state. */
@Composable
private fun SealedCard(
    entry: SealedEntry,
    zone: ZoneId,
    now: ExactInstant,
    resolvedCount: Int,
    open: Boolean,
    onToggle: () -> Unit,
    onResolve: (Boolean) -> Unit,
) {
    val r = entry.record
    val closed = r.windowEnd.isBefore(now)
    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).clickable(onClick = onToggle).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("${r.area} · ${r.kind}", style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text, modifier = Modifier.weight(1f))
            if (!r.verified) SealedTag("ALTERED", KshanaColors.Sky)
            when {
                !r.resolved -> SealedTag("OPEN", KshanaColors.Muted)
                r.happened -> SealedTag("HAPPENED", KshanaColors.Gold)
                else -> SealedTag("DID NOT", KshanaColors.Sky)
            }
        }
        Text(
            "${localText(r.windowStart, DATE_ONLY, zone)} – ${localText(r.windowEnd, DATE_ONLY, zone)} · sealed ${localText(r.createdAt, DATE_ONLY, zone)}",
            fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky,
        )
        if (resolvedCount >= MIN_FOR_PERCENT) {
            Text("%.0f%%".format(r.probability * 100.0), fontFamily = KshanaFonts.mono, fontSize = 20.sp, color = KshanaColors.Gold)
        } else {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text(sentence(probabilityWords(r.probability)), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
                Text("fit %.2f".format(r.probability), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Gold, modifier = Modifier.padding(start = 12.dp))
            }
        }
        Text(r.question, fontFamily = KshanaFonts.serif, fontSize = 16.sp, lineHeight = 22.sp, color = KshanaColors.Text)
        if (!r.verified) {
            Text("The stored claim no longer matches its id: it was edited after sealing, so it cannot count.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Sky)
        }
        when {
            r.resolved -> {
                val at = r.resolvedAt
                Text(
                    if (at != null) "resolved ${localText(at, DATE_ONLY, zone)}" else "resolved",
                    fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted,
                )
            }
            closed -> {
                Text("The window has closed. What happened?", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    QuietButton("HAPPENED", onClick = { onResolve(true) }, modifier = Modifier.weight(1f))
                    QuietButton("DID NOT", onClick = { onResolve(false) }, modifier = Modifier.weight(1f))
                }
            }
            else -> {
                Text(
                    if (r.windowStart.isBefore(now)) "The window is open; it closes ${localText(r.windowEnd, DATE_ONLY, zone)}."
                    else "The window opens ${localText(r.windowStart, DATE_ONLY, zone)}.",
                    style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
                )
            }
        }
        if (open) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            Text("BASIS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            for (b in r.basis) Text("· ${sentence(b)}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            Text("id ${r.id}", fontFamily = KshanaFonts.mono, fontSize = 9.sp, lineHeight = 13.sp, color = KshanaColors.Muted)
        } else {
            Text("id ${r.id.take(16)}… · tap for the basis", fontFamily = KshanaFonts.mono, fontSize = 9.sp, color = KshanaColors.Muted)
        }
    }
}

/** A small outlined tag, 22 dp. */
@Composable
private fun SealedTag(text: String, color: Color) {
    val shape = RoundedCornerShape(11.dp)
    Box(
        Modifier.height(22.dp).clip(shape).border(1.dp, color.copy(alpha = 0.6f), shape).padding(horizontal = 8.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(text, fontFamily = KshanaFonts.sans, fontSize = 9.sp, letterSpacing = 1.sp, color = color)
    }
}

/** The sealing form: area, kind, the window as two local dates. */
@Composable
private fun SealForm(
    kinds: List<String>,
    zone: ZoneId,
    sealing: Boolean,
    onSeal: (area: String, kind: String, start: ZonedDateTime, end: ZonedDateTime, onProblem: (String) -> Unit) -> Unit,
    onCancel: () -> Unit,
) {
    var area by remember { mutableStateOf(SEALED_AREAS[0]) }
    var kind by remember { mutableStateOf(kinds.first()) }
    var y0 by remember { mutableStateOf("") }
    var m0 by remember { mutableStateOf("") }
    var d0 by remember { mutableStateOf("") }
    var y1 by remember { mutableStateOf("") }
    var m1 by remember { mutableStateOf("") }
    var d1 by remember { mutableStateOf("") }
    var problem by remember { mutableStateOf<String?>(null) }

    fun submit() {
        val sy = y0.toIntOrNull(); val sm = m0.toIntOrNull(); val sd = d0.toIntOrNull()
        val ey = y1.toIntOrNull(); val em = m1.toIntOrNull(); val ed = d1.toIntOrNull()
        if (sy == null || sm == null || sd == null || ey == null || em == null || ed == null) {
            problem = "The window needs a start and an end, each a year, a month and a day."; return
        }
        val start: ZonedDateTime
        val end: ZonedDateTime
        try {
            start = ZonedDateTime.of(LocalDateTime.of(sy, sm, sd, 0, 0, 0), zone)
            end = ZonedDateTime.of(LocalDateTime.of(ey, em, ed, 23, 59, 59), zone)
        } catch (e: DateTimeException) {
            problem = "Those dates do not exist on the calendar."; return
        }
        if (!end.isAfter(start)) { problem = "The window must end after it starts."; return }
        problem = null
        onSeal(area, kind, start, end) { problem = it }
    }

    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("AREA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (a in SEALED_AREAS) Chip(a.uppercase(), selected = area == a, onClick = { area = a })
        }
        Text("KIND", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (k in kinds) Chip(k.uppercase(), selected = kind == k, onClick = { kind = k })
        }
        Text("WINDOW OPENS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            LedgerNumberField(y0, { y0 = it.filter(Char::isDigit).take(4) }, "Year", Modifier.weight(1.4f))
            LedgerNumberField(m0, { m0 = it.filter(Char::isDigit).take(2) }, "Month", Modifier.weight(1f))
            LedgerNumberField(d0, { d0 = it.filter(Char::isDigit).take(2) }, "Day", Modifier.weight(1f))
        }
        Text("WINDOW CLOSES", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            LedgerNumberField(y1, { y1 = it.filter(Char::isDigit).take(4) }, "Year", Modifier.weight(1.4f))
            LedgerNumberField(m1, { m1 = it.filter(Char::isDigit).take(2) }, "Month", Modifier.weight(1f))
            LedgerNumberField(d1, { d1 = it.filter(Char::isDigit).take(2) }, "Day", Modifier.weight(1f))
        }
        Text(
            "Dates are read in ${zone.id}: the window runs from the start of the first day to the end of the last. It must open after this moment; the engine refuses anything already open.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )
        problem?.let { Text(it, color = KshanaColors.Sky, style = MaterialTheme.typography.bodyMedium) }
        GoldButton(if (sealing) "SEALING…" else "SEAL", enabled = !sealing, onClick = { submit() }, modifier = Modifier.fillMaxWidth())
        QuietButton("CANCEL", onClick = onCancel, modifier = Modifier.fillMaxWidth())
    }
}

/** Brier and calibration over the resolved forecasts, numbers only when the engine's flags allow. */
@Composable
private fun SealedScore(m: MetricsRecord) {
    if (m.curveShown) {
        MonoRow("resolved", "${m.n} · ${m.positives} happened")
        MonoRow("base rate", "%.2f".format(m.baseRate))
        MonoRow("Brier", "%.3f".format(m.brier))
        MonoRow("always the base rate", "%.3f".format(m.brierReference))
        MonoRow("skill", "%+.2f".format(m.skill), valueColor = if (m.skill > 0.0) KshanaColors.Gold else KshanaColors.Sky)
        MonoRow("reliability · resolution", "%.3f · %.3f".format(m.reliability, m.resolution))
        MonoRow("ECE", "%.3f".format(m.ece))
        if (m.bins.any { it.shown }) {
            MetricsBinsChart(m.bins)
            Text(
                "Bars are the share of each bin that happened, with its 95% interval; the dot is the bin's mean probability. Bins with fewer than five forecasts are left blank.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
        }
    }
    Text(m.words, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
}
