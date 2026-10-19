package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.LedgerStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.sky.BreathingRing
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.CalibrationRecord
import uniffi.kshana_ffi.LedgerEventRecord
import uniffi.kshana_ffi.LedgerScoreRecord
import uniffi.kshana_ffi.PeriodReadingRecord
import uniffi.kshana_ffi.ReliabilityBinRecord
import uniffi.kshana_ffi.ledgerEventKinds
import java.time.DateTimeException
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.ZonedDateTime
import java.time.temporal.ChronoUnit

/** The eight life areas, in the engine's order (area strengths come back in this order). */
private val LEDGER_AREAS = listOf("Career", "Marriage", "Children", "Wealth", "Health", "Property", "Education", "Foreign")

/** Two-letter initials for the tiny bars: two areas start with C, so one letter would not do. */
private val AREA_SHORT = listOf("Ca", "Ma", "Ch", "We", "He", "Pr", "Ed", "Fo")

/** The event kinds the engine accepts, used only if the engine cannot be asked. */
private val FALLBACK_KINDS = listOf("Began", "Ended", "Gain", "Loss", "Milestone", "Crisis")

/** Importance chips: weight and label. */
private val IMPORTANCE = listOf(0.5 to "0.5 MINOR", 0.8 to "0.8 NOTABLE", 1.0 to "1.0 MAJOR")

/** Baseline instants scored per logged event when measuring. */
private val PER_EVENT: UInt = 12u

/** Weight factor when the time of day is unknown and the event sits at noon. */
private const val UNKNOWN_TIME_FACTOR = 0.8

private const val TAB_PAST = 0
private const val TAB_EVENTS = 1
private const val TAB_KNOW = 2

/** One antardasha of the past with its text precomputed off the main thread. */
private data class PastRow(val period: PeriodReadingRecord, val range: String, val ages: String, val decade: Int)

private fun ordinal(n: Int): String {
    val suffix = if (n % 100 in 11..13) "th" else when (n % 10) { 1 -> "st"; 2 -> "nd"; 3 -> "rd"; else -> "th" }
    return "$n$suffix"
}

private fun plural(n: Int, one: String, many: String = one + "s"): String = "$n ${if (n == 1) one else many}"

/**
 * The life ledger: what the chart said about every stretch of the past, the events the
 * user has logged with the chart's score for each, and an honest measure of whether the
 * chart scored those moments above moments nobody logged.
 */
@Composable
fun LedgerScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val zone = remember(profile) { profile.zone() }
    var tab by remember { mutableIntStateOf(TAB_PAST) }
    var events by remember(profile.name) { mutableStateOf(LedgerStore.events(context, profile.name)) }
    val kinds = remember { runCatching { ledgerEventKinds() }.getOrNull()?.takeIf { it.isNotEmpty() } ?: FALLBACK_KINDS }

    // Your past: every antardasha from birth to now, read once per profile and settings.
    val pastList = rememberLazyListState()
    var rows by remember(profile, settings) { mutableStateOf<List<PastRow>?>(null) }
    var pastError by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(profile, settings) {
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            val birthDate = profile.birthLocal().toLocalDate()
            rows = withContext(Dispatchers.Default) {
                val periods = engine.retrodict(birth, profile.latitude, profile.longitude, settings.ayanamsa, birth, Instants.now())
                periods.map { p ->
                    val s = Instants.toLocal(p.start, zone)
                    val e = Instants.toLocal(p.end, zone)
                    val a0 = ChronoUnit.YEARS.between(birthDate, s.toLocalDate()).toInt().coerceAtLeast(0)
                    val a1 = ChronoUnit.YEARS.between(birthDate, e.toLocalDate()).toInt().coerceAtLeast(0)
                    PastRow(
                        period = p,
                        range = "${s.format(DATE_ONLY)} – ${e.format(DATE_ONLY)}",
                        ages = if (a0 == a1) "age $a0" else "age $a0–$a1",
                        decade = a0 / 10 * 10,
                    )
                }
            }
            pastError = null
        } catch (e: Exception) {
            pastError = e.message ?: e.toString()
        }
    }

    // Your events: scores for the whole ledger whenever it changes, cached by the list's hash.
    val scoreCache = remember(profile, settings) { HashMap<Int, List<LedgerScoreRecord>>() }
    var scores by remember { mutableStateOf<List<LedgerScoreRecord>?>(null) }
    var scoresHash by remember { mutableIntStateOf(0) }
    var scoreError by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(events, profile, settings) {
        val list = events
        val hash = list.hashCode()
        val cached = scoreCache[hash]
        if (cached != null) {
            scores = cached; scoresHash = hash
            return@LaunchedEffect
        }
        if (list.isEmpty()) {
            scoreCache[hash] = emptyList()
            scores = emptyList(); scoresHash = hash
            return@LaunchedEffect
        }
        scores = null
        try {
            val engine = EngineProvider.get(context)
            val found = withContext(Dispatchers.Default) {
                engine.scoreEvents(profile.birthInstant(), profile.latitude, profile.longitude, settings.ayanamsa, list)
            }
            scoreCache[hash] = found
            scores = found; scoresHash = hash
            scoreError = null
        } catch (e: Exception) {
            scoreError = e.message ?: e.toString()
        }
    }
    val currentScores = if (scoresHash == events.hashCode()) scores else null

    // Did it know?: the calibration, run on demand.
    var calibration by remember(profile, settings) { mutableStateOf<CalibrationRecord?>(null) }
    var calibratedHash by remember { mutableIntStateOf(0) }
    var measuring by remember { mutableStateOf(false) }
    var calError by remember { mutableStateOf<String?>(null) }

    fun measure() {
        if (measuring || events.isEmpty()) return
        measuring = true
        calError = null
        val list = events
        scope.launch {
            try {
                val engine = EngineProvider.get(context)
                val birth = profile.birthInstant()
                val found = withContext(Dispatchers.Default) {
                    engine.calibrateLedger(birth, profile.latitude, profile.longitude, settings.ayanamsa, list, birth, Instants.now(), PER_EVENT)
                }
                calibration = found
                calibratedHash = list.hashCode()
            } catch (e: Exception) {
                calError = e.message ?: e.toString()
            }
            measuring = false
        }
    }

    Column(Modifier.fillMaxSize().padding(top = 48.dp)) {
        Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text("LIFE LEDGER", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
            }
            Text("Your life, on the record", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
            Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Chip("YOUR PAST", selected = tab == TAB_PAST, onClick = { tab = TAB_PAST })
                Chip("YOUR EVENTS", selected = tab == TAB_EVENTS, onClick = { tab = TAB_EVENTS })
                Chip("DID IT KNOW?", selected = tab == TAB_KNOW, onClick = { tab = TAB_KNOW })
            }
        }
        Box(Modifier.fillMaxWidth().weight(1f)) {
            when (tab) {
                TAB_PAST -> PastTab(rows, pastError, pastList)
                TAB_EVENTS -> EventsTab(
                    profile = profile,
                    zone = zone,
                    kinds = kinds,
                    events = events,
                    scores = currentScores,
                    scoreError = scoreError,
                    onAdd = { events = LedgerStore.add(context, profile.name, it) },
                    onRemove = { events = LedgerStore.remove(context, profile.name, it) },
                )
                else -> KnowTab(
                    events = events,
                    result = calibration,
                    stale = calibration != null && calibratedHash != events.hashCode(),
                    measuring = measuring,
                    error = calError,
                    onMeasure = { measure() },
                    onLog = { tab = TAB_EVENTS },
                )
            }
        }
    }
}

/* ------------------------------------------------------------------ Your past */

@Composable
private fun PastTab(rows: List<PastRow>?, error: String?, listState: LazyListState) {
    val scope = rememberCoroutineScope()
    Column(Modifier.fillMaxSize()) {
        Column(Modifier.padding(horizontal = 24.dp).padding(top = 10.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Text(
                "This is what your chart said about every stretch of your life, before you tell it anything.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
            )
            ErrorLine(error)
            if (rows == null && error == null) {
                Text("Reading every sub-period from your birth to today…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            } else if (rows != null && rows.isEmpty()) {
                Text("No sub-period fell between your birth and today.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            }
        }
        if (!rows.isNullOrEmpty()) {
            val decades = remember(rows) { rows.map { it.decade }.distinct() }
            val visibleDecade = rows.getOrNull(listState.firstVisibleItemIndex)?.decade
            SectionLabel("Decade", Modifier.padding(horizontal = 24.dp))
            Row(
                Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 24.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                for (d in decades) {
                    Chip("${d}s", selected = d == visibleDecade, onClick = {
                        val index = rows.indexOfFirst { it.decade == d }
                        if (index >= 0) scope.launch { listState.animateScrollToItem(index) }
                    })
                }
            }
            LazyColumn(
                modifier = Modifier.fillMaxWidth().weight(1f),
                state = listState,
                contentPadding = PaddingValues(start = 24.dp, end = 24.dp, top = 10.dp, bottom = 32.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                itemsIndexed(rows) { _, r -> PeriodCard(r) }
            }
        }
    }
}

/** One antardasha: lords in serif, the local range in mono, the words, eight tiny area bars. */
@Composable
private fun PeriodCard(r: PastRow) {
    val p = r.period
    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("${p.maha} · ${p.antar}", style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text, modifier = Modifier.weight(1f))
            val phase = p.sadeSatiPhase.toInt()
            if (phase > 0) Tag("SADE SATI $phase/3")
        }
        Text("${r.range} · ${r.ages}", fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky)
        Text(p.words, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        AreaBars(p.areaStrengths, p.topAreas)
    }
}

/** Eight tiny bars in area order, the leading areas in gold, initials beneath. */
@Composable
private fun AreaBars(strengths: List<Double>, top: List<String>) {
    Row(Modifier.fillMaxWidth().padding(top = 2.dp), horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.Bottom) {
        for (i in LEDGER_AREAS.indices) {
            val v = strengths.getOrNull(i)?.toFloat()?.coerceIn(0f, 1f) ?: 0f
            val lead = LEDGER_AREAS[i] in top
            Column(Modifier.weight(1f), horizontalAlignment = Alignment.CenterHorizontally) {
                Box(
                    Modifier.fillMaxWidth().height(28.dp).clip(RoundedCornerShape(2.dp)).background(KshanaColors.LineSoft),
                    contentAlignment = Alignment.BottomCenter,
                ) {
                    Box(
                        Modifier.fillMaxWidth().fillMaxHeight(v.coerceAtLeast(0.03f))
                            .background(if (lead) KshanaColors.Gold else KshanaColors.Sky.copy(alpha = 0.7f)),
                    )
                }
                Text(
                    AREA_SHORT[i], fontFamily = FontFamily.Monospace, fontSize = 9.sp,
                    color = if (lead) KshanaColors.Gold else KshanaColors.Muted, modifier = Modifier.padding(top = 2.dp),
                )
            }
        }
    }
}

/** A small outlined tag, 22 dp, for a state like a Sade Sati phase. */
@Composable
private fun Tag(text: String, color: Color = KshanaColors.Gold) {
    val shape = RoundedCornerShape(11.dp)
    Box(
        Modifier.height(22.dp).clip(shape).border(1.dp, color.copy(alpha = 0.6f), shape).padding(horizontal = 8.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(text, fontFamily = FontFamily.SansSerif, fontSize = 9.sp, letterSpacing = 1.sp, color = color)
    }
}

/* ---------------------------------------------------------------- Your events */

@Composable
private fun EventsTab(
    profile: BirthProfile,
    zone: ZoneId,
    kinds: List<String>,
    events: List<LedgerEventRecord>,
    scores: List<LedgerScoreRecord>?,
    scoreError: String?,
    onAdd: (LedgerEventRecord) -> Unit,
    onRemove: (Int) -> Unit,
) {
    var expanded by remember { mutableIntStateOf(-1) }
    var confirm by remember { mutableIntStateOf(-1) }
    var formOpen by remember { mutableStateOf(false) }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 10.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text(
            "What happened, and when. Each event is scored against the chart as it stood that day; tap one to see the score.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
        )
        if (events.isEmpty()) {
            Text(
                "No events yet. A marriage, a first job, a move abroad, an illness: anything with a date.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
        } else {
            if (scores == null && scoreError == null) {
                Text("Scoring ${plural(events.size, "event")} against the chart…", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            }
            ErrorLine(scoreError)
        }

        for ((i, e) in events.withIndex()) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            EventRow(
                e = e,
                score = scores?.getOrNull(i),
                zone = zone,
                open = expanded == i,
                confirming = confirm == i,
                onToggle = {
                    expanded = if (expanded == i) -1 else i
                    if (confirm != i) confirm = -1
                },
                onRemove = {
                    if (confirm == i) {
                        confirm = -1
                        expanded = -1
                        onRemove(i)
                    } else {
                        confirm = i
                    }
                },
            )
        }
        if (events.isNotEmpty()) HorizontalDivider(color = KshanaColors.LineSoft)

        Spacer(Modifier.height(4.dp))
        if (!formOpen) {
            GoldButton("LOG AN EVENT", onClick = { formOpen = true }, modifier = Modifier.fillMaxWidth())
        } else {
            EventForm(
                profile = profile,
                zone = zone,
                kinds = kinds,
                onSave = { onAdd(it); formOpen = false },
                onCancel = { formOpen = false },
            )
        }
    }
}

/** One logged event; open, it shows what the chart would have said and a two-tap remove. */
@Composable
private fun EventRow(
    e: LedgerEventRecord,
    score: LedgerScoreRecord?,
    zone: ZoneId,
    open: Boolean,
    confirming: Boolean,
    onToggle: () -> Unit,
    onRemove: () -> Unit,
) {
    Column(
        Modifier.fillMaxWidth().heightIn(min = 44.dp).clickable(onClick = onToggle).padding(vertical = 8.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text(e.note.trim().ifEmpty { "${e.kind} · ${e.area}" }, fontFamily = FontFamily.Serif, fontSize = 17.sp, color = KshanaColors.Text)
                Text(
                    "${localText(e.at, DATE_TIME, zone)} · ${e.area} · ${e.kind.lowercase()}",
                    style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
                )
            }
            Text(
                if (score != null) "%.2f".format(score.fit) else "…",
                fontFamily = FontFamily.Monospace, fontSize = 12.sp,
                color = if (score != null) KshanaColors.Gold else KshanaColors.Muted,
                modifier = Modifier.padding(start = 12.dp),
            )
            Text(if (open) "−" else "+", fontFamily = FontFamily.Monospace, fontSize = 14.sp, color = KshanaColors.Gold, modifier = Modifier.padding(start = 10.dp))
        }
        if (open) {
            if (score == null) {
                Text("Not scored yet.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            } else {
                ScorePanel(score)
            }
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text(
                    if (confirming) "Tap again and this event is gone." else "weight ${"%.2f".format(e.weight)}",
                    style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.weight(1f),
                )
                Box(Modifier.height(44.dp).clickable(onClick = onRemove).padding(horizontal = 12.dp), contentAlignment = Alignment.Center) {
                    Text(if (confirming) "REMOVE, REALLY" else "REMOVE", style = MaterialTheme.typography.labelSmall, color = if (confirming) KshanaColors.Gold else KshanaColors.Muted)
                }
            }
        }
    }
}

/** The fit as a breathing ring with the number inside, the running lords, the reasons, the words. */
@Composable
private fun ScorePanel(s: LedgerScoreRecord) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(14.dp), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(64.dp), contentAlignment = Alignment.Center) {
            BreathingRing(s.fit.toFloat(), Modifier.size(64.dp), color = KshanaColors.Gold)
            Text("%.2f".format(s.fit), fontFamily = FontFamily.Monospace, fontSize = 13.sp, color = KshanaColors.Text)
        }
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(3.dp)) {
            Text("FIT", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text(
                "running " + s.running.joinToString(" › ").ifEmpty { "—" },
                fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky,
            )
            Text(
                "dasha %.2f · parashari %.2f · transit %.2f%s".format(s.dashaSupport, s.parashari, s.transit, if (s.adverse) " · adverse" else ""),
                fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted,
            )
            val phase = s.sadeSatiPhase.toInt()
            Text(
                "Jupiter ${ordinal(s.jupiterFromMoon.toInt())} · Saturn ${ordinal(s.saturnFromMoon.toInt())} from the Moon" +
                    (if (phase > 0) " · Sade Sati $phase/3" else ""),
                fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted,
            )
        }
    }
    for (line in s.explanation) {
        Text("· ${sentence(line)}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
    }
    if (s.words.isNotBlank()) {
        Text(s.words, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
    }
}

/** The inline entry form: date, time or "unknown time", area, kind, importance, note. */
@Composable
private fun EventForm(
    profile: BirthProfile,
    zone: ZoneId,
    kinds: List<String>,
    onSave: (LedgerEventRecord) -> Unit,
    onCancel: () -> Unit,
) {
    var year by remember { mutableStateOf("") }
    var month by remember { mutableStateOf("") }
    var day by remember { mutableStateOf("") }
    var hour by remember { mutableStateOf("12") }
    var minute by remember { mutableStateOf("00") }
    var unknownTime by remember { mutableStateOf(false) }
    var area by remember { mutableStateOf(LEDGER_AREAS[0]) }
    var kind by remember { mutableStateOf(kinds.first()) }
    var importance by remember { mutableStateOf(0.8) }
    var note by remember { mutableStateOf("") }
    var problem by remember { mutableStateOf<String?>(null) }

    fun save() {
        val y = year.toIntOrNull(); val mo = month.toIntOrNull(); val d = day.toIntOrNull()
        if (y == null || mo == null || d == null) { problem = "The event needs a year, a month and a day."; return }
        val h = if (unknownTime) 12 else hour.toIntOrNull() ?: 12
        val mi = if (unknownTime) 0 else minute.toIntOrNull() ?: 0
        val local = try {
            ZonedDateTime.of(LocalDateTime.of(y, mo, d, h, mi, 0), zone)
        } catch (e: DateTimeException) {
            problem = "That date and time do not exist on the calendar."; return
        }
        if (local.isBefore(profile.birthLocal())) { problem = "That is before the birth; the ledger begins there."; return }
        if (local.isAfter(ZonedDateTime.now(zone))) { problem = "That is still ahead; the ledger is for what has already happened."; return }
        problem = null
        val weight = if (unknownTime) importance * UNKNOWN_TIME_FACTOR else importance
        onSave(LedgerEventRecord(Instants.fromInstant(local.toInstant()), area, kind, weight, note.trim()))
    }

    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("WHEN", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            LedgerNumberField(year, { year = it.filter(Char::isDigit).take(4) }, "Year", Modifier.weight(1.4f))
            LedgerNumberField(month, { month = it.filter(Char::isDigit).take(2) }, "Month", Modifier.weight(1f))
            LedgerNumberField(day, { day = it.filter(Char::isDigit).take(2) }, "Day", Modifier.weight(1f))
        }
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            LedgerNumberField(hour, { hour = it.filter(Char::isDigit).take(2) }, "Hour", Modifier.weight(1f), enabled = !unknownTime)
            LedgerNumberField(minute, { minute = it.filter(Char::isDigit).take(2) }, "Minute", Modifier.weight(1f), enabled = !unknownTime)
        }
        Row(Modifier.fillMaxWidth().heightIn(min = 44.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.SpaceBetween) {
            Column(Modifier.weight(1f)) {
                Text("Unknown time", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
                Text(
                    "Placed at 12:00 in ${profile.zoneId}; the event then counts ×0.8.",
                    style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
                )
            }
            Switch(
                checked = unknownTime,
                onCheckedChange = {
                    unknownTime = it
                    if (it) { hour = "12"; minute = "00" }
                },
                colors = SwitchDefaults.colors(
                    checkedThumbColor = KshanaColors.Night,
                    checkedTrackColor = KshanaColors.Gold,
                    uncheckedThumbColor = KshanaColors.Muted,
                    uncheckedTrackColor = KshanaColors.Surface,
                    uncheckedBorderColor = KshanaColors.Line,
                ),
            )
        }

        Text("AREA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (a in LEDGER_AREAS) Chip(a.uppercase(), selected = area == a, onClick = { area = a })
        }
        Text("KIND", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (k in kinds) Chip(k.uppercase(), selected = kind == k, onClick = { kind = k })
        }
        Text("IMPORTANCE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((w, label) in IMPORTANCE) Chip(label, selected = importance == w, onClick = { importance = w })
        }

        LedgerTextField(note, { note = it }, "Note (e.g. Married, First job)", Modifier.fillMaxWidth())
        problem?.let { Text(it, color = KshanaColors.Sky, style = MaterialTheme.typography.bodyMedium) }
        GoldButton("SAVE", onClick = { save() }, modifier = Modifier.fillMaxWidth())
        QuietButton("CANCEL", onClick = onCancel, modifier = Modifier.fillMaxWidth())
    }
}

/* --------------------------------------------------------------- Did it know? */

@Composable
private fun KnowTab(
    events: List<LedgerEventRecord>,
    result: CalibrationRecord?,
    stale: Boolean,
    measuring: Boolean,
    error: String?,
    onMeasure: () -> Unit,
    onLog: () -> Unit,
) {
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 10.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        if (events.isEmpty()) {
            Text(
                "Nothing to measure yet. Log at least one event under Your events and the chart can be checked against what actually happened.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
            )
            QuietButton("LOG AN EVENT", onClick = onLog, modifier = Modifier.fillMaxWidth())
            return@Column
        }

        Text(
            "Every logged event is scored, then ${PER_EVENT.toInt()} moments nobody logged are scored for each one, spread from your birth to today. It may take a minute.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
        )
        GoldButton(if (measuring) "MEASURING…" else "MEASURE", enabled = !measuring, onClick = onMeasure, modifier = Modifier.fillMaxWidth())
        if (measuring) {
            Text(
                "Scoring ${plural(events.size, "event")} and ${events.size * PER_EVENT.toInt()} quiet moments…",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
            )
        }
        ErrorLine(error)

        if (result != null) {
            if (stale) {
                Text("The ledger has changed since this was measured; measure again for the current list.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Gold)
            }
            SectionLabel("Lift")
            Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                Text("lift", fontFamily = FontFamily.Monospace, fontSize = 20.sp, color = KshanaColors.Muted, modifier = Modifier.padding(bottom = 6.dp))
                Text("%+.2f".format(result.lift), fontFamily = FontFamily.Monospace, fontWeight = FontWeight.Medium, fontSize = 44.sp, color = KshanaColors.Sky)
            }
            Text(result.verdict, fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Text)

            SectionLabel("Numbers")
            MonoRow("events", result.n.toString())
            MonoRow("baseline instants", result.baselineN.toString())
            MonoRow("mean fit", "%.2f".format(result.meanFit))
            MonoRow("base rate", "%.2f".format(result.baseRate))
            MonoRow("support rate", "%.0f%%".format(result.supportRate * 100.0))
            MonoRow("Brier", "%.3f".format(result.brier))

            SectionLabel("Reliability")
            ReliabilityChart(result.reliability)

            Spacer(Modifier.height(4.dp))
            Text(
                "Lift is how much higher the chart scored the moments you logged than moments nobody logged. Zero means it did no better than chance.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
            )
            Text(
                "The chart above sorts every scored moment, yours and the quiet ones, into five bins by its fit. The bar is the share of each bin that you actually logged; the dot sits at the bin's mean fit. If the chart knew, the high-fit bins would be mostly yours and the bars would climb the diagonal. Brier is the average squared miss, lower is better; 0.25 is what always answering 0.5 scores.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
            Text(
                "With a handful of events this says little either way. Log more, and the measure sharpens.",
                fontFamily = FontFamily.Serif, fontStyle = FontStyle.Italic, fontSize = 14.sp, lineHeight = 20.sp, color = KshanaColors.Muted,
            )
        }
    }
}

/**
 * Five reliability bins: a sky bar per bin at its observed rate (the share of the bin that
 * is a logged event), a gold dot at the bin's mean predicted fit, and the diagonal where a
 * perfectly calibrated score would sit.
 */
@Composable
private fun ReliabilityChart(bins: List<ReliabilityBinRecord>) {
    val measurer = rememberTextMeasurer()
    val label = TextStyle(fontFamily = FontFamily.Monospace, fontSize = 9.sp, color = KshanaColors.Muted)
    val labelGold = TextStyle(fontFamily = FontFamily.Monospace, fontSize = 9.sp, color = KshanaColors.Gold)
    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, KshanaColors.Line, shape).padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Canvas(Modifier.fillMaxWidth().height(180.dp)) {
            val left = 28.dp.toPx()
            val right = size.width - 6.dp.toPx()
            val top = 14.dp.toPx()
            val bottom = size.height - 16.dp.toPx()
            val w = right - left
            val h = bottom - top
            fun x(v: Double): Float = left + (v.coerceIn(0.0, 1.0) * w).toFloat()
            fun y(v: Double): Float = bottom - (v.coerceIn(0.0, 1.0) * h).toFloat()

            drawLine(KshanaColors.Line, Offset(left, top), Offset(left, bottom), strokeWidth = 1f)
            drawLine(KshanaColors.Line, Offset(left, bottom), Offset(right, bottom), strokeWidth = 1f)
            drawLine(
                KshanaColors.Line, Offset(x(0.0), y(0.0)), Offset(x(1.0), y(1.0)),
                strokeWidth = 1f, pathEffect = PathEffect.dashPathEffect(floatArrayOf(6f, 6f)),
            )
            for (v in listOf(0.0, 0.5, 1.0)) {
                val lay = measurer.measure("%.1f".format(v), label)
                drawText(lay, topLeft = Offset(left - lay.size.width - 4f, y(v) - lay.size.height / 2f))
            }

            val n = bins.size.coerceAtLeast(1)
            val binWidth = w / n
            val barWidth = binWidth * 0.5f
            for (b in bins) {
                val cx = x(b.center)
                val xl = measurer.measure("%.1f".format(b.center), label)
                drawText(xl, topLeft = Offset(cx - xl.size.width / 2f, bottom + 3f))
                if (b.count == 0u) continue
                val yTop = y(b.observedRate)
                drawRect(KshanaColors.Sky.copy(alpha = 0.55f), topLeft = Offset(cx - barWidth / 2f, yTop), size = Size(barWidth, bottom - yTop))
                drawCircle(KshanaColors.Gold, radius = 2.5.dp.toPx(), center = Offset(x(b.predictedMean), yTop))
                val cl = measurer.measure("n=${b.count}", labelGold)
                drawText(cl, topLeft = Offset(cx - cl.size.width / 2f, (yTop - cl.size.height - 2f).coerceAtLeast(0f)))
            }
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text("bins by fit →", fontFamily = FontFamily.Monospace, fontSize = 9.sp, color = KshanaColors.Muted)
            Text("bar share logged · dot mean fit", fontFamily = FontFamily.Monospace, fontSize = 9.sp, color = KshanaColors.Muted)
        }
    }
}

/* ------------------------------------------------------------------- fields */

@Composable
private fun LedgerTextField(value: String, onChange: (String) -> Unit, label: String, modifier: Modifier = Modifier) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label, style = MaterialTheme.typography.labelSmall) },
        singleLine = true,
        textStyle = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 15.sp, color = KshanaColors.Text),
        colors = ledgerFieldColors(),
        modifier = modifier,
    )
}

@Composable
private fun LedgerNumberField(value: String, onChange: (String) -> Unit, label: String, modifier: Modifier = Modifier, enabled: Boolean = true) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label, style = MaterialTheme.typography.labelSmall) },
        singleLine = true,
        enabled = enabled,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
        textStyle = TextStyle(fontFamily = FontFamily.Monospace, fontSize = 15.sp, color = KshanaColors.Text),
        colors = ledgerFieldColors(),
        modifier = modifier,
    )
}

@Composable
private fun ledgerFieldColors() = OutlinedTextFieldDefaults.colors(
    focusedTextColor = KshanaColors.Text,
    unfocusedTextColor = KshanaColors.Text,
    disabledTextColor = KshanaColors.Muted,
    cursorColor = KshanaColors.Gold,
    focusedBorderColor = KshanaColors.Gold,
    unfocusedBorderColor = KshanaColors.Line,
    disabledBorderColor = KshanaColors.LineSoft,
    focusedLabelColor = KshanaColors.Gold,
    unfocusedLabelColor = KshanaColors.Muted,
    disabledLabelColor = KshanaColors.Muted,
    focusedContainerColor = KshanaColors.Surface,
    unfocusedContainerColor = KshanaColors.Surface,
    disabledContainerColor = KshanaColors.Surface,
)
