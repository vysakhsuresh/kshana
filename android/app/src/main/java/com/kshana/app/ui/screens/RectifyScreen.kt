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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
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
import uniffi.kshana_ffi.CandidateRecord
import uniffi.kshana_ffi.LifeEventRecord
import uniffi.kshana_ffi.RectifyRecord
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter

private data class EventDraft(val date: LocalDate, val life: String, val good: Boolean)

private val LONG_DATE_TIME: DateTimeFormatter = DateTimeFormatter.ofPattern("d MMM uuuu · HH:mm:ss")

/**
 * Rectify the birth time from things that happened. The window is the stated
 * uncertainty (or ±15 minutes when the time was called exact); each candidate is cast
 * and scored by how well its periods and slow sky explain the events.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RectifyScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit, onSaved: (BirthProfile) -> Unit) {
    val context = LocalContext.current
    var events by remember { mutableStateOf(listOf<EventDraft>()) }
    var draftLife by remember { mutableStateOf("Love") }
    var draftGood by remember { mutableStateOf(true) }
    var pickDate by remember { mutableStateOf(false) }
    var halfHours by remember { mutableStateOf((profile.uncertaintySeconds().coerceAtLeast(900L)) / 3600.0) }
    var result by remember { mutableStateOf<RectifyRecord?>(null) }
    var running by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var request by remember { mutableStateOf<Long?>(null) }
    var showAll by remember { mutableStateOf(false) }

    LaunchedEffect(request) {
        if (request == null || events.isEmpty()) return@LaunchedEffect
        running = true
        try {
            val engine = EngineProvider.get(context)
            val centre = profile.birthLocal().toInstant()
            val half = (halfHours * 3600).toLong()
            val evs = events.map { e ->
                LifeEventRecord(Instants.fromInstant(e.date.atTime(12, 0).atZone(profile.zone()).toInstant()), e.life, e.good)
            }
            val max = ((2 * half) / 120L).toInt().coerceIn(7, 181)
            result = withContext(Dispatchers.Default) {
                engine.rectify(
                    profile.latitude, profile.longitude, settings.houseSystem, settings.ayanamsa,
                    Instants.fromInstant(centre.minusSeconds(half)), Instants.fromInstant(centre.plusSeconds(half)),
                    evs, max.toUInt(),
                )
            }
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
        running = false
    }

    if (pickDate) {
        val state = rememberDatePickerState(initialSelectedDateMillis = LocalDate.now().minusYears(1).atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli())
        DatePickerDialog(
            onDismissRequest = { pickDate = false },
            confirmButton = {
                TextButton(onClick = {
                    state.selectedDateMillis?.let { ms ->
                        val d = Instant.ofEpochMilli(ms).atZone(ZoneOffset.UTC).toLocalDate()
                        events = events + EventDraft(d, draftLife, draftGood)
                        result = null
                    }
                    pickDate = false
                }) { Text("ADD", color = KshanaColors.Gold) }
            },
            dismissButton = { TextButton(onClick = { pickDate = false }) { Text("CANCEL", color = KshanaColors.Muted) } },
        ) { DatePicker(state = state) }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("RECTIFY", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("Find the true minute", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "Name a few things that happened and when: a marriage, a move, a loss, a job. Every birth time inside your window is cast and scored by how well its periods and sky explain those days. The rising sign and the Moon's star are what the window can change; the answer shows which fits.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        Text("Stated birth: ${profile.birthLocal().format(LONG_DATE_TIME)} · ${profile.placeName}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)

        SectionLabel("The window")
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((label, h) in listOf("±15 MIN" to 0.25, "±30 MIN" to 0.5, "±1 H" to 1.0, "±2 H" to 2.0, "±3 H" to 3.0, "±6 H" to 6.0, "±12 H" to 12.0)) {
                Chip(label, selected = kotlin.math.abs(halfHours - h) < 1e-6, onClick = { halfHours = h; result = null })
            }
        }

        SectionLabel("What happened")
        for ((i, e) in events.withIndex()) {
            Row(Modifier.fillMaxWidth().padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                Text(e.date.format(DateTimeFormatter.ofPattern("d MMM uuuu")), fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky, modifier = Modifier.width(110.dp))
                Text("${lifeGlyph(e.life)} ${e.life} · ${if (e.good) "good" else "hard"}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text, modifier = Modifier.weight(1f))
                Text("REMOVE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable { events = events.filterIndexed { j, _ -> j != i }; result = null }.padding(6.dp))
            }
        }
        if (events.isEmpty()) Text("Nothing yet. Two to five dated events give a clear answer.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        Text("Add one", style = MaterialTheme.typography.titleMedium, color = KshanaColors.Gold)
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (l in LIFE_ORDER) Chip("${lifeGlyph(l)} ${l.uppercase()}", selected = draftLife == l, onClick = { draftLife = l })
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OptionBox("It was good", selected = draftGood, onClick = { draftGood = true }, modifier = Modifier.weight(1f))
            OptionBox("It was hard", selected = !draftGood, onClick = { draftGood = false }, modifier = Modifier.weight(1f))
        }
        QuietButton("PICK THE DATE AND ADD", onClick = { pickDate = true }, modifier = Modifier.fillMaxWidth())

        GoldButton(
            if (running) "CASTING EVERY MINUTE OF THE WINDOW…" else "FIND THE MINUTE",
            enabled = !running && events.isNotEmpty(),
            onClick = { request = System.currentTimeMillis() },
            modifier = Modifier.fillMaxWidth(),
        )
        ErrorLine(error)

        val r = result
        if (r != null) {
            SectionLabel("What the window can change")
            Text("The best time for each rising sign the window crosses. A higher fit explains more of what happened.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            for (c in r.bySign) CandidateRow(c, profile, best = c == r.bySign.firstOrNull(), onUse = { chosen ->
                val local = Instants.toLocal(chosen.birth, profile.zone())
                val p = profile.copy(hour = local.hour, minute = local.minute, second = local.second, uncertaintyLevel = 1)
                ProfileStore.save(context, p)
                onSaved(p)
            })
            Text(
                if (showAll) "FEWER −" else "EVERY CANDIDATE, ${r.candidates.size} +",
                style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                modifier = Modifier.clickable { showAll = !showAll }.padding(vertical = 4.dp),
            )
            if (showAll) {
                for (c in r.candidates.take(40)) {
                    HorizontalDivider(color = KshanaColors.LineSoft)
                    Row(Modifier.fillMaxWidth().padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                        Text(Instants.toLocal(c.birth, profile.zone()).format(DateTimeFormatter.ofPattern("HH:mm")), fontFamily = FontFamily.Monospace, fontSize = 13.sp, color = KshanaColors.Sky, modifier = Modifier.width(60.dp))
                        Text("${c.lagnaSign} rising · ${c.moonStar} · ${c.dashaAtBirth}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
                        Text(c.fit.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 14.sp, color = scoreColor(c.fit))
                    }
                }
            }
            Text("The fit is a reading of the chart against your own history, not a proof; two times that explain everything equally are both kept. Use the one that also matches what you know of your rising sign.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
    }
}

@Composable
private fun CandidateRow(c: CandidateRecord, profile: BirthProfile, best: Boolean, onUse: (CandidateRecord) -> Unit) {
    Column(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp))
            .background(if (best) KshanaColors.Gold.copy(alpha = 0.10f) else KshanaColors.Surface)
            .then(if (best) Modifier.border(1.dp, KshanaColors.Gold, RoundedCornerShape(12.dp)) else Modifier)
            .padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(Instants.toLocal(c.birth, profile.zone()).format(DateTimeFormatter.ofPattern("HH:mm:ss")), fontFamily = FontFamily.Monospace, fontSize = 20.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
            Text("fit ${c.fit.toInt()}", fontFamily = FontFamily.Monospace, fontSize = 14.sp, color = scoreColor(c.fit))
        }
        Text(c.words, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        Box(Modifier.fillMaxWidth().height(4.dp).clip(RoundedCornerShape(2.dp)).background(KshanaColors.Line)) {
            Box(Modifier.fillMaxWidth((c.fit / 100.0).toFloat().coerceIn(0.02f, 1f)).height(4.dp).background(scoreColor(c.fit)))
        }
        Spacer(Modifier.height(2.dp))
        QuietButton("USE THIS TIME", onClick = { onUse(c) }, modifier = Modifier.fillMaxWidth())
    }
}
