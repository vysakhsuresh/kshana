package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Journal
import com.kshana.app.data.LifeEvent
import com.kshana.app.data.Place
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter

/** How the app read each journal day, computed once per process. */
internal object JournalCache {
    val scores = HashMap<String, Double>()
}

/**
 * Your journal: what actually happened, in your words. Each entry is set beside what
 * the app read for that part of your life on that day, so the app's record is shown
 * openly, and the entries sharpen your birth time.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun JournalScreen(profile: BirthProfile, place: Place, settings: EngineSettings, onBack: () -> Unit, onRectify: () -> Unit, onDay: (LocalDate) -> Unit) {
    val context = LocalContext.current
    var version by remember { mutableStateOf(0) }
    val events = remember(version) { Journal.all(context) }
    var date by remember { mutableStateOf(LocalDate.now()) }
    var life by remember { mutableStateOf("Love") }
    var good by remember { mutableStateOf(false) }
    var note by remember { mutableStateOf("") }
    var pick by remember { mutableStateOf(false) }
    var scores by remember { mutableStateOf<Map<Long, Double>>(emptyMap()) }

    LaunchedEffect(version, profile) {
        val out = HashMap<Long, Double>()
        for (e in events) {
            val k = "${profile.birthInstant().taiSeconds}|${e.date}|${e.life}"
            val s = JournalCache.scores[k] ?: runCatching {
                val noon = e.date.atTime(12, 0).atZone(ZoneId.systemDefault()).toInstant()
                val m = loadMoment(context, profile, place, settings, Instants.fromInstant(noon))
                m.lives.firstOrNull { it.life == e.life }?.score
            }.getOrNull()?.also { JournalCache.scores[k] = it }
            if (s != null) out[e.id] = s
        }
        scores = out
    }

    if (pick) {
        val state = rememberDatePickerState(initialSelectedDateMillis = date.atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli())
        DatePickerDialog(
            onDismissRequest = { pick = false },
            confirmButton = {
                TextButton(onClick = {
                    state.selectedDateMillis?.let { date = Instant.ofEpochMilli(it).atZone(ZoneOffset.UTC).toLocalDate() }
                    pick = false
                }) { Text("SET", color = KshanaColors.Gold) }
            },
            dismissButton = { TextButton(onClick = { pick = false }) { Text("CANCEL", color = KshanaColors.Muted) } },
        ) { DatePicker(state = state, title = null, headline = null, showModeToggle = false) }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("YOUR JOURNAL", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("What really happened", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "Write down the days that mattered: a job, a quarrel, an illness, a move, good news. Each one is set beside what the app read for that day, so you can see for yourself how often it is right. Three or more also sharpen your birth time.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )

        // The proof, in one line.
        val graded = events.filter { scores.containsKey(it.id) }
        if (graded.isNotEmpty()) {
            val agree = graded.count { agrees(it, scores.getValue(it.id)) }
            Column(Modifier.fillMaxWidth().glass(16.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text("HOW OFTEN THE APP MATCHED YOUR LIFE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text("$agree of ${graded.size}", fontFamily = FontFamily.Monospace, fontSize = 30.sp, color = KshanaColors.Text)
                Text(
                    "On $agree of your ${graded.size} days, the reading for that part of life pointed the same way as what happened. Nothing is hidden: the ones it got wrong are marked below.",
                    style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
                )
            }
        }

        SectionLabel("Add a day")
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            ValuePill(date.format(DateTimeFormatter.ofPattern("d MMM uuuu")), onClick = { pick = true })
            OptionBox("It was good", selected = good, onClick = { good = true })
            OptionBox("It was hard", selected = !good, onClick = { good = false })
        }
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (l in LIFE_ORDER) Chip("${lifeGlyph(l)} ${lifeName(l).uppercase()}", selected = life == l, onClick = { life = l })
        }
        OutlinedTextField(
            value = note, onValueChange = { note = it.take(200) },
            label = { Text("In a few words (optional)") },
            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences),
            colors = OutlinedTextFieldDefaults.colors(focusedBorderColor = KshanaColors.Gold, unfocusedBorderColor = KshanaColors.Line, focusedLabelColor = KshanaColors.Gold),
            modifier = Modifier.fillMaxWidth(),
        )
        GoldButton("WRITE IT DOWN", onClick = {
            Journal.add(context, date, life, good, note)
            note = ""
            version++
            com.kshana.app.data.Backup.autoSave(context)
        }, modifier = Modifier.fillMaxWidth())

        if (events.size >= 3) {
            QuietButton("USE THESE TO FIND MY EXACT BIRTH TIME", onClick = onRectify, modifier = Modifier.fillMaxWidth())
        }

        if (events.isNotEmpty()) SectionLabel("Your days · ${events.size}")
        for (e in events) {
            val s = scores[e.id]
            Column(
                Modifier.fillMaxWidth().glass(14.dp).clickable { onDay(e.date) }.padding(14.dp),
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("${lifeGlyph(e.life)} ${lifeName(e.life)} · ${if (e.good) "good" else "hard"}", style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text, modifier = Modifier.weight(1f))
                    Text(e.date.format(DateTimeFormatter.ofPattern("d MMM uuuu")), fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky)
                }
                if (e.note.isNotBlank()) Text(e.note, fontFamily = FontFamily.Serif, fontSize = 16.sp, color = KshanaColors.TextSoft)
                if (s == null) {
                    Text("reading that day…", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                } else {
                    val ok = agrees(e, s)
                    Text(
                        (if (ok) "✓ Matched. " else "✗ Did not match. ") +
                            "The app read ${lifeName(e.life).lowercase()} at ${s.toInt()} (${bandWord(bandOf(s)).lowercase()}) that day.",
                        style = MaterialTheme.typography.bodySmall, color = if (ok) KshanaColors.Leaf else KshanaColors.Ember,
                    )
                }
                Row {
                    Text("SEE THAT DAY ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.weight(1f))
                    Text("REMOVE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable { Journal.remove(context, e.id); version++ })
                }
            }
        }
        Spacer(Modifier.width(1.dp))
        Text("Kept on this phone and in your backup file. Nothing leaves it.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    }
}

/** A good day agrees with a reading of 50 or more; a hard day with less than 50. */
internal fun agrees(e: LifeEvent, score: Double): Boolean = if (e.good) score >= 50.0 else score < 50.0

internal fun bandOf(score: Double): String = when {
    score >= 70 -> "bright"; score >= 56 -> "good"; score >= 44 -> "steady"; score >= 32 -> "slow"; else -> "hold"
}
