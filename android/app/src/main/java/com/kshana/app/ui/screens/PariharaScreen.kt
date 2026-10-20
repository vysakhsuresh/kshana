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
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
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
import androidx.compose.runtime.mutableStateMapOf
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
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.Commitment
import com.kshana.app.data.CommitmentStore
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Expert
import com.kshana.app.data.Metrics
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.sky.KshanaFonts
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.DoshaRecord
import uniffi.kshana_ffi.PariharaRecord
import uniffi.kshana_ffi.RemedyRecord
import uniffi.kshana_ffi.RemedyWindowRecord
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.temporal.ChronoUnit

private val ACTIVE_KINDS = setOf("Karma", "Seva", "Discipline")

private fun kindColour(kind: String): Color = when (kind) {
    in ACTIVE_KINDS -> KshanaColors.Gold
    "Daana", "Vrata" -> KshanaColors.Sky
    "Anna", "Jeeva", "Prakriti" -> KshanaColors.Leaf
    "Vastra" -> KshanaColors.Sky
    else -> KshanaColors.Muted
}

private fun kindWords(kind: String): String = kindWord(kind)

private val GROUPS = listOf(
    "ALL" to null,
    "DO" to setOf("Karma", "Seva", "Discipline"),
    "FEED" to setOf("Anna", "Jeeva"),
    "GIVE" to setOf("Daana", "Prakriti", "Vastra"),
    "FAST & PRAYER" to setOf("Vrata", "Mantra", "Pooja"),
)

private fun planetDay(planet: String): java.time.DayOfWeek? = when (planet) {
    "Sun" -> java.time.DayOfWeek.SUNDAY
    "Moon" -> java.time.DayOfWeek.MONDAY
    "Mars", "Ketu" -> java.time.DayOfWeek.TUESDAY
    "Mercury" -> java.time.DayOfWeek.WEDNESDAY
    "Jupiter" -> java.time.DayOfWeek.THURSDAY
    "Venus" -> java.time.DayOfWeek.FRIDAY
    "Saturn", "Rahu" -> java.time.DayOfWeek.SATURDAY
    else -> null
}

/** The eight life areas, in the engine's order; "All" asks for the running lords alone. */
private val AREAS = listOf("Career", "Marriage", "Children", "Wealth", "Health", "Property", "Education", "Foreign")
private const val ALL = "All"

private val DAY_WORDS = DateTimeFormatter.ofPattern("EEEE d MMM uuuu")
private val WHEN_WORDS = DateTimeFormatter.ofPattern("EEEE d MMM uuuu, HH:mm:ss")

/** The ladder's rungs, as the engine numbers them. */
private fun rungLabel(rung: Int): String = when (rung) {
    1 -> "Karma"
    2 -> "Seva"
    3 -> "Vrata and discipline"
    4 -> "Daana"
    5 -> "Mantra"
    6 -> "Pooja"
    7 -> "Gem, informational"
    else -> "Rung $rung"
}

private fun dayWords(millis: Long, zone: ZoneId): String = Instant.ofEpochMilli(millis).atZone(zone).format(DAY_WORDS)
private fun whenWords(millis: Long, zone: ZoneId): String = Instant.ofEpochMilli(millis).atZone(zone).format(WHEN_WORDS)

/**
 * What will help. Everyone sees the plain page: what is pressing, the remedies in plain
 * words, and the effort kept. Only with "Show the chart pages" on (Expert) can the person
 * open the full remedy ledger: the cause, karma class, doshas, rungs and exact windows.
 */
@Composable
fun PariharaScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit, life: String? = null) {
    val context = LocalContext.current
    val expert = remember { Expert.on(context) }
    var full by remember { mutableStateOf(false) }
    if (expert && full) {
        PariharaLedgerScreen(profile, settings, onBack = { full = false }, life = life)
    } else {
        PariharaPlainScreen(profile, settings, onBack, life, onFull = if (expert) ({ full = true }) else null)
    }
}

/** What will help: the cause, the ranked remedies, and the commitments kept so far. */
@Composable
private fun PariharaPlainScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit, life: String?, onFull: (() -> Unit)?) {
    val context = LocalContext.current
    var record by remember { mutableStateOf<PariharaRecord?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var kept by remember { mutableStateOf<Set<String>>(emptySet()) }
    var since by remember { mutableStateOf<LocalDate?>(null) }

    LaunchedEffect(profile, settings, life) {
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            val rec = withContext(Dispatchers.Default) {
                engine.parihara(birth, profile.latitude, profile.longitude, settings.ayanamsa, Instants.now(), life)
            }
            val prefs = ProfileStore.prefs(context)
            kept = rec.remedies.map { it.title }.filter { prefs.getBoolean(commitKey(rec.planet, it), false) }.toSet()
            since = prefs.getLong(sinceKey(rec.planet), 0L).takeIf { it > 0L }?.let { LocalDate.ofEpochDay(it) }
            record = rec
            error = null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    fun toggle(rec: PariharaRecord, r: RemedyRecord) {
        val prefs = ProfileStore.prefs(context)
        val now = r.title !in kept
        kept = if (now) kept + r.title else kept - r.title
        val edit = prefs.edit().putBoolean(commitKey(rec.planet, r.title), now)
        if (kept.isEmpty()) {
            since = null
            edit.remove(sinceKey(rec.planet))
        } else if (since == null) {
            val today = LocalDate.now()
            since = today
            edit.putLong(sinceKey(rec.planet), today.toEpochDay())
        }
        edit.apply()
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("WHAT TO DO", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("What will help", style = MaterialTheme.typography.displayLarge, color = KshanaColors.Text)
        if (onFull != null) {
            Text(
                "OPEN THE FULL REMEDY LEDGER",
                style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                modifier = Modifier.clickable(onClick = onFull).padding(vertical = 8.dp),
            )
        }
        ErrorLine(error)
        val rec = record
        if (rec == null) {
            if (error == null) Working("finding what will help…")
        } else {
            val required = rec.remedies.filter { !it.optional }
            val keptRequired = required.count { it.title in kept }
            val worthiness = if (required.isEmpty()) 0f else keptRequired.toFloat() / required.size

            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                WorthinessRing(worthiness)
                Spacer(Modifier.width(20.dp))
                Column {
                    Text("YOUR EFFORT", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                    Text("$keptRequired / ${required.size}", style = MaterialTheme.typography.headlineLarge, color = KshanaColors.Text)
                    Text("of the main remedies you are keeping", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                }
            }
            Text(
                since?.let { d ->
                    val days = ChronoUnit.DAYS.between(d, LocalDate.now()) + 1
                    "day $days of keeping it up · ${kept.size} remedies held"
                } ?: "No commitment yet. Tap a remedy to hold it.",
                fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky,
            )

            SectionLabel("What is pressing on you")
            Column(Modifier.fillMaxWidth().glass(8.dp).padding(16.dp)) {
                Text(when { rec.severity >= 0.66 -> "PRESSING HARD"; rec.severity >= 0.33 -> "PRESSING MODERATELY"; else -> "PRESSING LIGHTLY" }, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text(rec.cause, fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Text, modifier = Modifier.padding(top = 6.dp))
            }

            // The streak: today's tick on each held remedy, and the longest run.
            var streakVersion by remember { mutableStateOf(0) }
            val bestStreak = remember(streakVersion, kept) { com.kshana.app.data.StreakStore.best(context, rec.planet, kept.toList()) }
            val today = java.time.LocalDate.now().dayOfWeek
            val pd = planetDay(rec.planet)
            if (pd == today) {
                Text("Today is ${today.getDisplayName(java.time.format.TextStyle.FULL, java.util.Locale.ENGLISH)}, the day these remedies work best. What you keep today counts double; tick what you did.", fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Gold)
            }
            if (kept.isNotEmpty()) {
                Row(Modifier.fillMaxWidth().glass(12.dp).padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text("🔥", fontSize = 22.sp)
                    Spacer(Modifier.width(10.dp))
                    Column(Modifier.weight(1f)) {
                        Text("$bestStreak-day streak", style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text)
                        Text("Tick TODAY on a held remedy once you have done it. The run counts consecutive days; a morning tick keeps yesterday's run alive.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                    }
                }
            }
            var group by remember { mutableStateOf("ALL") }
            Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for ((label, _) in GROUPS) Chip(label, selected = group == label, onClick = { group = label })
            }
            val set = GROUPS.firstOrNull { it.first == group }?.second
            val shown = rec.remedies.withIndex().filter { set == null || it.value.kind in set }
            SectionLabel("What helps most, first · ${shown.size} of ${rec.remedies.size}")
            if (shown.isEmpty()) Text("None of this kind for you right now.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            for ((i, r) in shown) RemedyRow(i + 1, r, r.title in kept, rec.planet, streakVersion, { streakVersion++ }) { toggle(rec, r) }
            Text("Doing, serving and good habits do the real work. Giving and fasting support them. Chanting and prayer are optional, never required.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(top = 8.dp))
        }
    }
}

@Composable
private fun WorthinessRing(fraction: Float) {
    Box(Modifier.size(112.dp), contentAlignment = Alignment.Center) {
        Canvas(Modifier.size(112.dp)) {
            val stroke = 8.dp.toPx()
            val inset = stroke / 2 + 2.dp.toPx()
            val arcSize = Size(size.width - 2 * inset, size.height - 2 * inset)
            drawArc(KshanaColors.LineSoft, -90f, 360f, false, topLeft = Offset(inset, inset), size = arcSize, style = Stroke(stroke))
            if (fraction > 0f) {
                drawArc(KshanaColors.Gold, -90f, 360f * fraction.coerceIn(0f, 1f), false, topLeft = Offset(inset, inset), size = arcSize, style = Stroke(stroke, cap = StrokeCap.Round))
            }
        }
        Text("${(fraction * 100).toInt()}%", fontFamily = FontFamily.Monospace, fontSize = 20.sp, color = KshanaColors.Text)
    }
}

@Composable
private fun RemedyRow(rank: Int, r: RemedyRecord, committed: Boolean, planet: String, streakVersion: Int, onTicked: () -> Unit, onToggle: () -> Unit) {
    val context = LocalContext.current
    val colour = kindColour(r.kind)
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(
        Modifier.fillMaxWidth().heightIn(min = 44.dp).clickable(onClick = onToggle).padding(vertical = 10.dp),
        verticalAlignment = Alignment.Top,
    ) {
        Text(rank.toString(), fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Muted, modifier = Modifier.width(22.dp).padding(top = 3.dp))
        Column(Modifier.weight(1f)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(kindWords(r.kind), style = MaterialTheme.typography.labelSmall, color = colour)
                if (r.optional) Text(" · OPTIONAL · NOT REQUIRED", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
            }
            Text(r.title, fontFamily = FontFamily.Serif, fontSize = 18.sp, color = if (r.optional) KshanaColors.TextSoft else KshanaColors.Text)
            Text(r.why, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            Text("${r.effort} · ${r.day}", fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky, modifier = Modifier.padding(top = 2.dp))
            if (committed) {
                val ticked = remember(streakVersion) { com.kshana.app.data.StreakStore.tickedToday(context, planet, r.title) }
                val streak = remember(streakVersion) { com.kshana.app.data.StreakStore.streak(context, planet, r.title) }
                Row(Modifier.padding(top = 6.dp), horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.CenterVertically) {
                    Chip(if (ticked) "✓ DONE TODAY" else "TODAY", selected = ticked, onClick = { com.kshana.app.data.StreakStore.toggleToday(context, planet, r.title); onTicked() })
                    if (streak > 0) Text("🔥 $streak", fontFamily = FontFamily.Monospace, fontSize = 13.sp, color = KshanaColors.Gold)
                }
            }
        }
        Box(
            Modifier.padding(start = 12.dp, top = 6.dp).size(22.dp).clip(CircleShape)
                .background(if (committed) KshanaColors.Gold else KshanaColors.LineSoft),
            contentAlignment = Alignment.Center,
        ) {
            if (committed) Text("✓", fontSize = 13.sp, color = KshanaColors.Night)
        }
    }
}

private fun commitKey(planet: String, title: String) = "parihara:$planet:$title"
private fun sinceKey(planet: String) = "parihara:since:$planet"

/* ---------------------------------------------------------------------------------- */

/**
 * "Earn the period.": the cause the engine locates, its karma class and what a remedy
 * may claim, the doshas the chart carries, the remedies ranked by rung with their next
 * exact windows, and the parihara ledger of what you planned and what you did.
 * A chart page: reached only from the plain page when Expert ("Show the chart pages") is on.
 */
@Composable
fun PariharaLedgerScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit, life: String? = null) {
    val context = LocalContext.current
    val zone = remember { ZoneId.systemDefault() }
    var area by remember(profile, life) { mutableStateOf(AREAS.firstOrNull { it.equals(life, ignoreCase = true) } ?: ALL) }
    val cache = remember(profile, settings) { mutableStateMapOf<String, PariharaRecord>() }
    var loading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var commitments by remember(profile) { mutableStateOf(CommitmentStore.commitments(context, profile.name)) }

    LaunchedEffect(profile, settings, area) {
        if (cache.containsKey(area)) return@LaunchedEffect
        loading = true
        try {
            val engine = EngineProvider.get(context)
            val birth = profile.birthInstant()
            val rec = withContext(Dispatchers.Default) {
                engine.parihara(birth, profile.latitude, profile.longitude, settings.ayanamsa, Instants.now(), if (area == ALL) null else area)
            }
            cache[area] = rec
            commitments = CommitmentStore.refreshPlans(context, profile.name, rec.remedies)
            error = null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
        loading = false
    }

    val rec = cache[area]
    val nowMillis = System.currentTimeMillis()
    val metrics = remember(commitments, nowMillis / 60_000L) { CommitmentStore.metrics(commitments, nowMillis) }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text("PARIHARA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                QuietButton("BACK", onClick = onBack)
            }
            Text("Earn the period.", style = MaterialTheme.typography.displayLarge, color = KshanaColors.Text)
            Text(
                "The cause the running lords locate, what the texts let a remedy claim, and the ledger of what you did · ${profile.placeName}",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
            ErrorLine(error)
        }

        Row(Modifier.horizontalScroll(rememberScrollState()).padding(horizontal = 24.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (a in listOf(ALL) + AREAS) Chip(a.uppercase(), selected = a == area, onClick = { area = a })
        }

        Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            SectionLabel("The ledger")
            LedgerCard(metrics, commitments, rec, nowMillis, zone)

            if (rec == null) {
                if (loading || error == null) {
                    Text(
                        if (area == ALL) "weighing the running lords…" else "weighing the running lords and the lord of ${area.lowercase()}…",
                        style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
                    )
                }
            } else {
                SectionLabel("The cause")
                CauseCard(rec, zone)

                SectionLabel("Doshas present")
                val present = rec.doshas.filter { it.present }
                if (present.isEmpty()) {
                    Text("No curse yoga or dosha stands in this chart.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                } else {
                    for (d in present) DoshaRow(d)
                }

                SectionLabel("Remedies, ranked")
                if (rec.remedies.isEmpty()) {
                    Text("The engine names no remedy for this period.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                }
                var lastRung = -1
                for (r in rec.remedies) {
                    val rung = r.rung.toInt()
                    if (rung != lastRung) {
                        lastRung = rung
                        Text(
                            "$rung · ${rungLabel(rung)}".uppercase(),
                            style = MaterialTheme.typography.labelSmall,
                            color = if (rung == 7) KshanaColors.Muted else KshanaColors.Gold,
                            modifier = Modifier.padding(top = 8.dp),
                        )
                    }
                    RemedyCard(
                        r = r,
                        held = commitments.firstOrNull { it.ruleId == r.ruleId },
                        zone = zone,
                        nowMillis = nowMillis,
                        onHold = { commitments = CommitmentStore.commit(context, profile.name, r) },
                        onDone = { commitments = CommitmentStore.markDone(context, profile.name, r.ruleId) },
                        onRelease = { commitments = CommitmentStore.release(context, profile.name, r.ruleId) },
                    )
                }

                Text(rec.ledgerHint, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(top = 8.dp))
            }
            Text(
                "Every window is an exact instant from the solved sunrise; every remedy names its source. Times in ${zone.id}.",
                fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted,
                modifier = Modifier.padding(vertical = 8.dp),
            )
        }
    }
}

/* ---------------------------------------------------------------------------------- */

/** The night card every block sits in. */
@Composable
private fun NightCard(dim: Boolean = false, content: @Composable () -> Unit) {
    val shape = RoundedCornerShape(8.dp)
    Column(
        Modifier.fillMaxWidth().clip(shape).background(KshanaColors.Surface).border(1.dp, if (dim) KshanaColors.LineSoft else KshanaColors.Line, shape).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) { content() }
}

/** A 3 dp bar: the track in LineSoft, the fraction in [colour]. */
@Composable
private fun ThinBar(fraction: Float, colour: Color, modifier: Modifier = Modifier) {
    Box(modifier.fillMaxWidth().height(3.dp).clip(RoundedCornerShape(2.dp)).background(KshanaColors.LineSoft)) {
        val f = fraction.coerceIn(0f, 1f)
        if (f > 0f) Box(Modifier.fillMaxWidth(f).height(3.dp).background(colour))
    }
}

/** "WHY" / "HIDE": a quiet toggle in the label style. */
@Composable
private fun ToggleLabel(open: Boolean, label: String, onToggle: () -> Unit) {
    Text(
        if (open) "HIDE" else label.uppercase(),
        style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
        modifier = Modifier.clickable(onClick = onToggle).padding(vertical = 8.dp),
    )
}

/* ---------------------------------------------------------------------------------- */

/** Kept over planned for 7 and 40 days, the lapse and its repair, and the clock the logs come from. */
@Composable
private fun LedgerCard(m: Metrics, commitments: List<Commitment>, rec: PariharaRecord?, nowMillis: Long, zone: ZoneId) {
    NightCard {
        if (commitments.isEmpty()) {
            Text("You hold no remedy yet.", fontFamily = KshanaFonts.serif, fontSize = 18.sp, lineHeight = 24.sp, color = KshanaColors.Text)
            Text("Hold one below and the ledger opens at its first window.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            return@NightCard
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(16.dp)) {
            LedgerCount("7 DAYS", m.kept7, m.planned7, Modifier.weight(1f))
            LedgerCount("40 DAYS", m.kept40, m.planned40, Modifier.weight(1f))
        }
        Text("kept over planned · a log counts as kept; no streak is counted", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
        HorizontalDivider(color = KshanaColors.LineSoft)

        // The next window: the ledger's own plan first, else the engine's next window for a held remedy.
        val next = CommitmentStore.nextPlanned(commitments, nowMillis)
            ?: rec?.remedies
                ?.filter { r -> commitments.any { it.ruleId == r.ruleId } }
                ?.flatMap { it.windows }
                ?.map { CommitmentStore.millisOf(it.start) }
                ?.filter { it > nowMillis }
                ?.minOrNull()
        val nextWords = next?.let { "The next window is ${whenWords(it, zone)}." }
            ?: "No further window is planned; open this screen after the next sunrise and the plan extends."
        val lapse = m.lapses.lastOrNull()
        Text(
            when {
                lapse != null -> "You missed ${dayWords(lapse, zone)}. $nextWords Repair it there; one missed day does not void the vow."
                m.planned40 == 0 -> "Nothing has come due yet. $nextWords"
                else -> "You missed nothing in the last 40 days. $nextWords"
            },
            fontFamily = KshanaFonts.serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Text,
        )
        Text(
            "${commitments.size} held · logs from the ${CommitmentStore.DEVICE_CLOCK} · windows from the engine",
            fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted,
        )
    }
}

@Composable
private fun LedgerCount(label: String, kept: Int, planned: Int, modifier: Modifier = Modifier) {
    Column(modifier, verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(label, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        Row(verticalAlignment = Alignment.Bottom) {
            Text(kept.toString(), fontFamily = KshanaFonts.mono, fontSize = 28.sp, color = KshanaColors.Text)
            Text(" / $planned", fontFamily = KshanaFonts.mono, fontSize = 14.sp, color = KshanaColors.Sky, modifier = Modifier.padding(bottom = 4.dp))
        }
        ThinBar(if (planned == 0) 0f else kept.toFloat() / planned.toFloat(), KshanaColors.Gold)
    }
}

/* ---------------------------------------------------------------------------------- */

/** The planet, the cause in words, where it stands, and the karma verdict with its basis. */
@Composable
private fun CauseCard(rec: PariharaRecord, zone: ZoneId) {
    var why by remember(rec) { mutableStateOf(false) }
    NightCard {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(PLANET_GLYPHS[rec.planet] ?: rec.planet.take(2), fontSize = 28.sp, color = KshanaColors.Gold)
            Column {
                Text(rec.planet, style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text)
                Text("severity ${"%.2f".format(rec.severity)}", fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky)
            }
        }
        Text(rec.cause, fontFamily = KshanaFonts.serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Text)
        val running = rec.runningLevel?.let { level ->
            val since = rec.runningSince?.let { " since ${localText(it, DATE_ONLY, zone)}" } ?: ""
            val until = rec.runningUntil?.let { " until ${localText(it, DATE_ONLY, zone)}" } ?: ""
            " · running $level$since$until"
        } ?: ""
        Text(
            "house ${rec.causeHouse.toInt()} · placed in ${rec.placedIn.toInt()} · ${rec.dignity.lowercase()}$running",
            fontFamily = KshanaFonts.mono, fontSize = 11.sp, lineHeight = 16.sp, color = KshanaColors.Sky,
        )
        HorizontalDivider(color = KshanaColors.LineSoft)

        // The karma verdict: class and kind as a chip, then what a remedy may claim.
        val shape = RoundedCornerShape(16.dp)
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Box(
                Modifier.clip(shape).border(1.dp, KshanaColors.Gold, shape).padding(horizontal = 12.dp, vertical = 6.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(rec.karmaClass.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            }
            Text(rec.karmaKind.lowercase(), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Muted)
        }
        Text(sentence(rec.mayClaim), fontFamily = KshanaFonts.serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Gold)
        if (rec.karmaBasis.isNotEmpty()) {
            ToggleLabel(why, "why") { why = !why }
            if (why) {
                for (b in rec.karmaBasis) {
                    Row(verticalAlignment = Alignment.Top) {
                        Text("•", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Gold, modifier = Modifier.width(16.dp))
                        Text(sentence(b), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.weight(1f))
                    }
                }
            }
        }
        Text(rec.karmaSource, fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted)
    }
}

/** One present dosha: name, its severity as a thin bar, the offended being, factors on request, the source. */
@Composable
private fun DoshaRow(d: DoshaRecord) {
    var open by remember(d.ruleId) { mutableStateOf(false) }
    NightCard {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(d.name, fontFamily = KshanaFonts.serif, fontSize = 17.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
            Text("${PLANET_GLYPHS[d.planet] ?: ""} ${"%.2f".format(d.severity)}", fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky)
        }
        ThinBar(d.severity.toFloat(), if (d.severity >= 0.5) KshanaColors.Gold else KshanaColors.Sky)
        if (d.offended.isNotBlank()) {
            Text("Offended: ${d.offended}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        }
        if (d.remedyFamily.isNotBlank()) {
            Text("remedy family · ${d.remedyFamily}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
        }
        if (d.factors.isNotEmpty()) {
            ToggleLabel(open, "factors") { open = !open }
            if (open) {
                for (f in d.factors) {
                    Row(verticalAlignment = Alignment.Top) {
                        Text("•", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Gold, modifier = Modifier.width(16.dp))
                        Text(sentence(f), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
                    }
                }
            }
        }
        Text("${d.source} · ${d.ruleId}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted)
    }
}

/* ---------------------------------------------------------------------------------- */

/**
 * One remedy: kind, title, why, what it may claim, its cadence, the next three exact
 * windows each with a reminder, what the ledger records, and the hold / did it / release
 * control. Gems carry no control: information only.
 */
@Composable
private fun RemedyCard(
    r: RemedyRecord,
    held: Commitment?,
    zone: ZoneId,
    nowMillis: Long,
    onHold: () -> Unit,
    onDone: () -> Unit,
    onRelease: () -> Unit,
) {
    val gem = r.kind == "Gem"
    val colour = kindColour(r.kind)
    val textColour = if (gem) KshanaColors.Muted else if (r.optional) KshanaColors.TextSoft else KshanaColors.Text
    NightCard(dim = gem) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(r.kind.uppercase(), style = MaterialTheme.typography.labelSmall, color = colour)
            if (gem) {
                Text(" · INFORMATION ONLY", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
            } else if (r.optional) {
                Text(" · OPTIONAL, NEVER REQUIRED", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
            }
            Spacer(Modifier.weight(1f))
            Text("fit ${"%.2f".format(r.fit)} · ${r.effort}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
        }
        Text(r.title, fontFamily = KshanaFonts.serif, fontSize = 18.sp, lineHeight = 24.sp, color = textColour)
        Text(r.why, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        Text(
            sentence(r.mayClaim),
            fontFamily = KshanaFonts.serif, fontSize = 14.sp, lineHeight = 20.sp, fontStyle = FontStyle.Italic,
            color = if (gem) KshanaColors.Muted else KshanaColors.Gold,
        )
        val dose = buildList {
            add(r.cadence)
            r.count?.let { add("count $it") }
            r.durationDays?.let { add("$it days") }
        }.joinToString(" · ")
        Text(dose, fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = if (gem) KshanaColors.Muted else KshanaColors.Sky)

        if (!gem && r.windows.isNotEmpty()) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            Text("NEXT WINDOWS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
            for (w in r.windows.take(3)) WindowRow(r, w, zone)
        }

        Text(r.measurable, fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted)
        Text("${r.source} · ${r.ruleId}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Muted)

        if (gem) return@NightCard
        HorizontalDivider(color = KshanaColors.LineSoft)
        if (held == null) {
            QuietButton("HOLD THIS", onClick = onHold, modifier = Modifier.fillMaxWidth())
        } else {
            val last = held.doneInstants.lastOrNull()
            Text(
                "held since ${dayWords(held.startedAtMillis, zone)} · ${held.doneInstants.size} logged" +
                    (last?.let { " · last ${whenWords(it, zone)}" } ?: "") +
                    " · ${held.plannedInstants.count { it > nowMillis }} windows ahead",
                fontFamily = KshanaFonts.mono, fontSize = 10.sp, lineHeight = 15.sp, color = KshanaColors.Gold,
            )
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                GoldButton("DID IT NOW", onClick = onDone, modifier = Modifier.weight(1f))
                QuietButton("RELEASE", onClick = onRelease)
            }
        }
    }
}

/** One exact window: local start and end to the second, the words, and a reminder at the start. */
@Composable
private fun WindowRow(r: RemedyRecord, w: RemedyWindowRecord, zone: ZoneId) {
    val start = Instants.toLocal(w.start, zone)
    val end = Instants.toLocal(w.end, zone)
    val endText = if (start.toLocalDate() == end.toLocalDate()) end.format(TIME_ONLY) else end.format(DATE_TIME_S)
    val avoid = w.kind.contains("avoid")
    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text(
                    "${start.format(DATE_TIME_S)} – $endText",
                    fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = if (avoid) KshanaColors.Muted else KshanaColors.Sky,
                )
                Text(
                    "${w.kind}${if (w.horaIndex.toInt() > 0) " · hora ${w.horaIndex.toInt()}" else ""} · ${w.words}",
                    style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
                )
            }
            if (!avoid) {
                RemindButton(
                    key = "${r.ruleId}@${w.start.taiSeconds}",
                    atMillis = CommitmentStore.millisOf(w.start),
                    title = r.title,
                    text = w.words,
                    modifier = Modifier.padding(start = 8.dp),
                )
            }
        }
    }
}
