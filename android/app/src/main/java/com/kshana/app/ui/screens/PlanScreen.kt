package com.kshana.app.ui.screens

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
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TimePicker
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.material3.rememberTimePickerState
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
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.AppointmentRecord
import uniffi.kshana_ffi.PurposeRecord
import uniffi.kshana_ffi.purposes
import java.time.Instant
import java.time.LocalDate
import java.time.LocalTime
import java.time.ZoneId
import java.time.ZoneOffset
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter

private val DAY_LONG: DateTimeFormatter get() = com.kshana.app.ui.screens.datePattern("EEEE d MMMM uuuu")

/**
 * Plan: "I have something at this time; how is that time for me?" Pick the moment and
 * the purpose, read the verdict, the reasons, and the better hours and days when the
 * time is poor. Nothing is typed; nothing leaves the phone.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PlanScreen(
    profile: BirthProfile,
    place: Place,
    settings: EngineSettings,
    onLife: (String) -> Unit = {},
    onRemedy: (String) -> Unit = {},
) {
    val context = LocalContext.current
    val zone: ZoneId = remember(place) { runCatching { com.kshana.app.data.CityClocks.of(place.zoneId) }.getOrDefault(ZoneId.systemDefault()) }
    val ps: List<PurposeRecord> = remember(com.kshana.app.data.Lang.code) { purposes(com.kshana.app.data.Lang.code) }
    val startLocal = remember { ZonedDateTime.now(zone).plusHours(1).withMinute(0).withSecond(0).withNano(0) }
    var date by remember { mutableStateOf(startLocal.toLocalDate()) }
    var time by remember { mutableStateOf(startLocal.toLocalTime()) }
    var purpose by remember { mutableStateOf(PlanPrefill.purpose?.also { PlanPrefill.purpose = null } ?: ps.firstOrNull()?.key ?: "meeting") }
    var pickDate by remember { mutableStateOf(false) }
    var pickTime by remember { mutableStateOf(false) }
    var result by remember { mutableStateOf<AppointmentRecord?>(null) }
    var reading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var request by remember { mutableStateOf<String?>(null) }
    var showLives by remember { mutableStateOf(false) }
    // Readings kept side by side: two times, or two places, for the same purpose.
    var kept by remember { mutableStateOf(listOf<Pair<String, AppointmentRecord>>()) }
    // The place the appointment is at; the current place by default.
    var planPlace by remember { mutableStateOf(place) }
    var bestWeek by remember { mutableStateOf<List<uniffi.kshana_ffi.DaySlotRecord>?>(null) }
    var findingBest by remember { mutableStateOf(false) }

    val chosen: ZonedDateTime = ZonedDateTime.of(date, time, zone)

    LaunchedEffect(request, com.kshana.app.data.Lang.code) {
        val r = request ?: return@LaunchedEffect
        reading = true
        try {
            val engine = EngineProvider.get(context)
            val at = Instants.fromInstant(chosen.toInstant())
            result = withContext(Dispatchers.Default) {
                engine.appointment(
                    profile.birthInstant(), profile.latitude, profile.longitude,
                    planPlace.latitude, planPlace.longitude,
                    settings.houseSystem, settings.ayanamsa, purpose, at, com.kshana.app.data.Lang.code,
                )
            }
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
        reading = false
        showLives = false
    }

    if (pickDate) {
        val state = rememberDatePickerState(initialSelectedDateMillis = date.atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli())
        DatePickerDialog(
            onDismissRequest = { pickDate = false },
            confirmButton = {
                TextButton(onClick = {
                    state.selectedDateMillis?.let { date = Instant.ofEpochMilli(it).atZone(ZoneOffset.UTC).toLocalDate() }
                    pickDate = false
                    result = null
                }) { Text("SET", color = KshanaColors.Gold) }
            },
            dismissButton = { TextButton(onClick = { pickDate = false }) { Text("CANCEL", color = KshanaColors.Muted) } },
        ) { DatePicker(state = state, title = null, headline = null, showModeToggle = false) }
    }
    if (pickTime) {
        val state = rememberTimePickerState(initialHour = time.hour, initialMinute = time.minute, is24Hour = true)
        AlertDialog(
            onDismissRequest = { pickTime = false },
            confirmButton = {
                TextButton(onClick = {
                    time = LocalTime.of(state.hour, state.minute)
                    pickTime = false
                    result = null
                }) { Text("SET", color = KshanaColors.Gold) }
            },
            dismissButton = { TextButton(onClick = { pickTime = false }) { Text("CANCEL", color = KshanaColors.Muted) } },
            text = { TimePicker(state = state) },
            containerColor = KshanaColors.Surface,
        )
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("PLAN", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(tr("How is that time for me?"), style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "A meeting, an interview, a journey, a date. You may not know what will be said or who will be there; this tells you how that hour will go for you. Pick the time and what it is for.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )

        SectionLabel(tr("When"))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            ValuePill(chosen.format(WEEKDAY_DAY), onClick = { pickDate = true }, modifier = Modifier.weight(1.4f))
            ValuePill(chosen.format(TIME_HM), onClick = { pickTime = true }, modifier = Modifier.weight(1f))
        }
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((label, action) in listOf<Pair<String, () -> Unit>>(
                "NOW" to { val n = ZonedDateTime.now(zone); date = n.toLocalDate(); time = n.toLocalTime().withSecond(0).withNano(0) },
                "+1 H" to { val n = chosen.plusHours(1); date = n.toLocalDate(); time = n.toLocalTime() },
                "TOMORROW" to { date = date.plusDays(1) },
                "+7 D" to { date = date.plusDays(7) },
                "MORNING 9:00" to { time = LocalTime.of(9, 0) },
                "NOON 12:00" to { time = LocalTime.of(12, 0) },
                "EVENING 18:00" to { time = LocalTime.of(18, 0) },
            )) {
                Text(
                    label, fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky,
                    modifier = Modifier.glass(10.dp).clickable { action(); result = null }.padding(horizontal = 10.dp, vertical = 8.dp),
                )
            }
        }
        Text("${chosen.format(DAY_LONG)} · ${place.name} time", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)

        SectionLabel(tr("For"))
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            for (row in ps.chunked(2)) {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    for (p in row) {
                        OptionBox(tr(p.words), selected = purpose == p.key, onClick = { purpose = p.key; result = null }, modifier = Modifier.weight(1f))
                    }
                    if (row.size == 1) Spacer(Modifier.weight(1f))
                }
            }
        }
        SectionLabel("Where")
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            val choices = (listOf(place, profile.place()) + com.kshana.app.data.ProfileStore.places).distinctBy { it.name }
            for (p in choices) Chip(p.name.uppercase(), selected = planPlace.name == p.name, onClick = { planPlace = p; result = null })
        }
        GoldButton(
            if (reading) "READING THAT HOUR…" else "READ · ${ps.firstOrNull { it.key == purpose }?.words?.uppercase() ?: purpose.uppercase()}",
            enabled = !reading,
            onClick = { request = "$purpose|$chosen|${planPlace.name}|${System.currentTimeMillis()}" },
            modifier = Modifier.fillMaxWidth(),
        )
        QuietButton(
            if (findingBest) "SCANNING SEVEN DAYS…" else "BEST HOURS IN THE NEXT 7 DAYS FOR THIS",
            onClick = {
                if (findingBest) return@QuietButton
                findingBest = true
                bestWeek = null
            },
            modifier = Modifier.fillMaxWidth(),
        )
        LaunchedEffect(findingBest, purpose, planPlace, com.kshana.app.data.Lang.code) {
            if (!findingBest) return@LaunchedEffect
            try {
                val engine = EngineProvider.get(context)
                bestWeek = withContext(Dispatchers.Default) {
                    engine.bestSlots(profile.birthInstant(), profile.latitude, profile.longitude, planPlace.latitude, planPlace.longitude, settings.houseSystem, settings.ayanamsa, purpose, Instants.now(), 7u, 5u, com.kshana.app.data.Lang.code)
                }
            } catch (e: Exception) {
                error = e.message ?: e.toString()
            }
            findingBest = false
        }
        ErrorLine(error)
        val bw = bestWeek
        if (bw != null) {
            SectionLabel("The week's best hours for ${ps.firstOrNull { it.key == purpose }?.words?.lowercase() ?: purpose}")
            for (s in bw) {
                Row(Modifier.fillMaxWidth().clickable {
                    val l = Instants.toLocal(s.start, zone)
                    date = l.toLocalDate(); time = l.toLocalTime().withSecond(0).withNano(0); result = null
                }.padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text("${localText(s.start, WEEKDAY_DAY, zone)} ${localText(s.start, TIME_HM, zone)} – ${localText(s.end, TIME_HM, zone)}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky, modifier = Modifier.weight(1f))
                    Text(hourShort(s.lord), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                    Spacer(Modifier.width(10.dp))
                    Text(s.score.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 15.sp, color = scoreColor(s.score))
                }
            }
            Text("The best hours for this in the coming week. Tap one to set it above and see it in full.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }

        if (kept.isNotEmpty()) {
            SectionLabel("Side by side")
            Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for ((label, r) in kept) {
                    Column(
                        Modifier.width(150.dp).glass(12.dp).padding(12.dp),
                        verticalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Text(label, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, maxLines = 2)
                        Text(r.score.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 28.sp, color = scoreColor(r.score))
                        Text(r.verdict, style = MaterialTheme.typography.titleMedium, color = scoreColor(r.score))
                        Text(r.purposeWords, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                        Text("DROP", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable { kept = kept.filter { it.second !== r } }.padding(vertical = 4.dp))
                    }
                }
            }
            val best = kept.maxByOrNull { it.second.score }
            if (best != null && kept.size > 1) Text("Best of these: ${best.first} at ${best.second.score.toInt()}.", fontFamily = FontFamily.Serif, fontSize = 17.sp, color = KshanaColors.Text)
        }

        val a = result
        if (a != null) {
            QuietButton("KEEP THIS, TO COMPARE", onClick = {
                val label = "${localText(a.at, WEEKDAY_DAY, zone)} ${localText(a.at, TIME_HM, zone)} · ${planPlace.name}"
                if (kept.none { it.first == label }) kept = (kept + (label to a)).takeLast(4)
            }, modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(4.dp))
            Column(
                Modifier.fillMaxWidth().glass(18.dp).padding(18.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                    ScoreRing(a.score, size = 84.dp, stroke = 7.dp, label = bandWord(a.band))
                    Column(Modifier.weight(1f)) {
                        Text("${a.purposeWords} · ${localText(a.at, WEEKDAY_DAY, zone)} ${localText(a.at, TIME_HM, zone)}", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                        Text(a.verdict, style = MaterialTheme.typography.displayMedium, color = scoreColor(a.score))
                    }
                }
                Text(a.headline, fontFamily = FontFamily.Serif, fontSize = 20.sp, lineHeight = 28.sp, color = KshanaColors.Text)
                a.caution?.let { label ->
                    val w = a.moment.windows.firstOrNull { it.label == label }
                    Text(
                        if (w != null) trf("That day has a {0} from {1} to {2}. Finish things in it, do not begin; start after {2}.", label.lowercase(), localText(w.start, TIME_HM, zone), localText(w.end, TIME_HM, zone))
                        else trf("That time falls in a {0}. Finish things, do not begin.", label.lowercase()),
                        style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Ember,
                    )
                }
                if (a.abhijit) Text(tr("These are the luckiest minutes of the day: a winning time to begin."), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Gold)
                HorizontalDivider(color = KshanaColors.LineSoft)
                for (line in a.advice) {
                    Row(verticalAlignment = Alignment.Top) {
                        Box(Modifier.padding(top = 7.dp).size(6.dp).clip(CircleShape).background(KshanaColors.Sky))
                        Spacer(Modifier.width(10.dp))
                        Text(line.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                    }
                }
            }

            SectionLabel("What decides it")
            for (l in a.lives) {
                Row(Modifier.fillMaxWidth().clickable { onLife(l.life) }.padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text("${lifeGlyph(l.life)} ${l.life}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text, modifier = Modifier.width(110.dp))
                    Box(Modifier.weight(1f).height(6.dp).clip(RoundedCornerShape(3.dp)).background(KshanaColors.Line)) {
                        Box(Modifier.fillMaxWidth((l.score / 100.0).toFloat().coerceIn(0.02f, 1f)).height(6.dp).background(scoreColor(l.score)))
                    }
                    Spacer(Modifier.width(10.dp))
                    Text(l.score.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 13.sp, color = scoreColor(l.score), modifier = Modifier.width(28.dp))
                    Text("×${"%.0f".format(l.weight * 100)}%", fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted)
                }
            }

            if (a.betterToday.isNotEmpty()) {
                SectionLabel("Better hours that day")
                for (s in a.betterToday) {
                    Row(Modifier.fillMaxWidth().padding(vertical = 3.dp), verticalAlignment = Alignment.CenterVertically) {
                        Text("${localText(s.start, TIME_HM, zone)} – ${localText(s.end, TIME_HM, zone)}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky, modifier = Modifier.width(110.dp))
                        Text(hourNature(s.lord).replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text, modifier = Modifier.weight(1f))
                        Text(s.score.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 14.sp, color = scoreColor(s.score))
                    }
                }
                Text("Each one avoids the day's unlucky stretches.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            }
            val bd = a.betterDay
            if (bd != null) {
                SectionLabel("A better day")
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(Modifier.size(8.dp).clip(CircleShape).background(KshanaColors.Gold))
                    Spacer(Modifier.width(10.dp))
                    Text(localText(bd, WEEKDAY_DAY, zone), fontFamily = FontFamily.Monospace, fontSize = 14.sp, color = KshanaColors.Gold, modifier = Modifier.width(110.dp))
                    Text(a.betterDayWords, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
                }
            }

            // A weak time, or one with nothing better that day, is not left bare: one deed to
            // carry into it, and the way to the remedies for the part of life it leans on hardest.
            if (a.score < 50 || (a.betterToday.isEmpty() && a.score < 62)) {
                val weakest = a.lives.minByOrNull { it.score }?.life
                Column(
                    Modifier.fillMaxWidth().glass(14.dp).padding(14.dp),
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    Text("IF YOU MUST GO", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                    Text(
                        "Before you set out, give something small to someone who needs it: food, a coin, or ten minutes of your help. Then go well prepared, a little early, with a calm word ready. A time asked for care still goes well when it is met with care.",
                        style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
                    )
                    if (weakest != null) {
                        Text(
                            "WHAT TO DO FOR ${lifeName(weakest).uppercase()} ›",
                            style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                            modifier = Modifier.clickable { onRemedy(weakest) }.padding(vertical = 4.dp),
                        )
                    }
                }
            }

            Text(
                if (showLives) "LESS −" else "EVERY PART OF YOUR LIFE AT THAT TIME +",
                style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                modifier = Modifier.clickable { showLives = !showLives }.padding(vertical = 4.dp),
            )
            if (showLives) {
                val m = a.moment
                Text(m.headline, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                for (l in m.lives) {
                    HorizontalDivider(color = KshanaColors.LineSoft)
                    Row(Modifier.fillMaxWidth().clickable { onLife(l.life) }.padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
                        Text(lifeGlyph(l.life), fontSize = 14.sp, color = bandColor(l.band), modifier = Modifier.width(22.dp))
                        Column(Modifier.weight(1f)) {
                            Text("${l.life} · ${bandWord(l.band)}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
                            Text(l.headline, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                        }
                        Text(l.score.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 16.sp, color = scoreColor(l.score))
                    }
                }
                Text(m.taraWords.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            }
            Text("Read for ${place.name}. Worked out on this phone; nothing leaves it.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
    }
}

/** The purpose Plan should open on, when a question asked for a good day for it. */
internal object PlanPrefill {
    var purpose: String? = null
}
