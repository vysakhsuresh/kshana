package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
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
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.PariharaRecord
import uniffi.kshana_ffi.RemedyRecord
import java.time.LocalDate
import java.time.temporal.ChronoUnit

private val ACTIVE_KINDS = setOf("Karma", "Seva", "Discipline")

private fun kindColour(kind: String): Color = when (kind) {
    in ACTIVE_KINDS -> KshanaColors.Gold
    "Daana", "Vrata" -> KshanaColors.Sky
    "Anna", "Jeeva", "Prakriti" -> KshanaColors.Leaf
    "Vastra" -> KshanaColors.Sky
    else -> KshanaColors.Muted
}

private fun kindWords(kind: String): String = when (kind) {
    "Anna" -> "FOOD"
    "Jeeva" -> "CREATURES"
    "Prakriti" -> "WATER & TREES"
    "Vastra" -> "COLOUR"
    else -> kind.uppercase()
}

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

/** "Earn the period.": the cause, the ranked remedies, and the commitments kept so far. */
@Composable
fun PariharaScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit, life: String? = null) {
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
            Text("PARIHARA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("Earn the period.", style = MaterialTheme.typography.displayLarge, color = KshanaColors.Text)
        ErrorLine(error)
        val rec = record
        if (rec == null) {
            if (error == null) Text("weighing the running lords…", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        } else {
            val required = rec.remedies.filter { !it.optional }
            val keptRequired = required.count { it.title in kept }
            val worthiness = if (required.isEmpty()) 0f else keptRequired.toFloat() / required.size

            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                WorthinessRing(worthiness)
                Spacer(Modifier.width(20.dp))
                Column {
                    Text("WORTHINESS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                    Text("$keptRequired / ${required.size}", style = MaterialTheme.typography.headlineLarge, color = KshanaColors.Text)
                    Text("commitments kept of those required", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                }
            }
            Text(
                since?.let { d ->
                    val days = ChronoUnit.DAYS.between(d, LocalDate.now()) + 1
                    "day $days of commitment · ${kept.size} held"
                } ?: "No commitment yet. Tap a remedy to hold it.",
                fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky,
            )

            SectionLabel("Cause")
            Column(Modifier.fillMaxWidth().clip(RoundedCornerShape(8.dp)).background(KshanaColors.Surface).padding(16.dp)) {
                Text("${rec.planet} · severity ${"%.2f".format(rec.severity)}", fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Gold)
                Text(rec.cause, fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Text, modifier = Modifier.padding(top = 6.dp))
                Text(rec.signature, fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Sky, modifier = Modifier.padding(top = 8.dp))
            }

            // The streak: today's tick on each held remedy, and the longest run.
            var streakVersion by remember { mutableStateOf(0) }
            val bestStreak = remember(streakVersion, kept) { com.kshana.app.data.StreakStore.best(context, rec.planet, kept.toList()) }
            val today = java.time.LocalDate.now().dayOfWeek
            val pd = planetDay(rec.planet)
            if (pd == today) {
                Text("Today is ${rec.planet}'s day. What you keep today counts double in the tradition; tick what you did.", fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Gold)
            }
            if (kept.isNotEmpty()) {
                Row(Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).background(KshanaColors.Surface).padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
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
            SectionLabel("Remedies, ranked · ${shown.size} of ${rec.remedies.size} in the dictionary for this")
            if (shown.isEmpty()) Text("none of this kind for this period", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            for ((i, r) in shown) RemedyRow(i + 1, r, r.title in kept, rec.planet, streakVersion, { streakVersion++ }) { toggle(rec, r) }
            Text("Karma, seva and discipline are the work. Daana and vrata support it. Mantra and pooja are optional and never required.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(top = 8.dp))
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
            Text("${r.effort} · ${r.day} · fit ${(r.fit * 100).toInt()}", fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky, modifier = Modifier.padding(top = 2.dp))
            Text(r.source, fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted)
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
