package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.runtime.Composable
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
import com.kshana.app.data.Integrity
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import uniffi.kshana_ffi.BlindMatchRecord
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter

/**
 * What happened? Blind logging, diary first: the person writes the event (the part of
 * life, the day, good or hard, a line) without seeing anything that was predicted. Only
 * after it is saved does the app look for sealed predictions whose dates cover that day
 * and part of life, and show them: "This was sealed on 21 Oct: …". A closed one the event
 * agrees with is answered by it, since it was written blind. Route: "whathappened".
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WhatHappenedScreen(profile: BirthProfile, onBack: () -> Unit, onScore: () -> Unit, onRemedy: () -> Unit) {
    val context = LocalContext.current
    var date by remember { mutableStateOf(LocalDate.now()) }
    var life by remember { mutableStateOf<String?>(null) }
    var good by remember { mutableStateOf<Boolean?>(null) }
    var note by remember { mutableStateOf("") }
    var pick by remember { mutableStateOf(false) }
    var saved by remember { mutableStateOf<List<BlindMatchRecord>?>(null) }
    var savedHard by remember { mutableStateOf(false) }

    if (pick) {
        val state = rememberDatePickerState(initialSelectedDateMillis = date.atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli())
        DatePickerDialog(
            onDismissRequest = { pick = false },
            confirmButton = {
                TextButton(onClick = {
                    state.selectedDateMillis?.let { date = Instant.ofEpochMilli(it).atZone(ZoneOffset.UTC).toLocalDate().coerceAtMost(LocalDate.now()) }
                    pick = false
                }) { Text(tr("SET"), color = KshanaColors.Gold) }
            },
            dismissButton = { TextButton(onClick = { pick = false }) { Text(tr("CANCEL"), color = KshanaColors.Muted) } },
        ) { DatePicker(state = state, title = null, headline = null, showModeToggle = false) }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text(tr("WHAT HAPPENED?"), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text(tr("BACK"), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        val result = saved
        if (result == null) {
            Text(tr("Tell it in your own words first"), style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
            Text(
                tr("Write down something that happened. Nothing that was predicted is shown until you have saved it, so what you write is yours alone. Afterwards, if anything sealed before that day covers it, you will see it."),
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
            SectionLabel(tr("Which part of life"))
            Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (l in LIFE_ORDER) Chip("${lifeGlyph(l)} ${lifeName(l).uppercase()}", selected = life == l, onClick = { life = l })
            }
            SectionLabel(tr("When"))
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                ValuePill(date.format(DateTimeFormatter.ofPattern("d MMM uuuu")), onClick = { pick = true })
            }
            SectionLabel(tr("How was it"))
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OptionBox(tr("It was good"), selected = good == true, onClick = { good = true }, modifier = Modifier.weight(1f))
                OptionBox(tr("It was hard"), selected = good == false, onClick = { good = false }, modifier = Modifier.weight(1f))
            }
            OutlinedTextField(
                value = note, onValueChange = { note = it.take(200) },
                label = { Text(tr("In a line (optional)")) },
                keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences),
                colors = OutlinedTextFieldDefaults.colors(focusedBorderColor = KshanaColors.Gold, unfocusedBorderColor = KshanaColors.Line, focusedLabelColor = KshanaColors.Gold),
                modifier = Modifier.fillMaxWidth(),
            )
            val l = life
            val g = good
            GoldButton(if (l == null || g == null) tr("PICK A PART OF LIFE AND GOOD OR HARD") else tr("SAVE IT"), onClick = {
                if (l != null && g != null) {
                    saved = Integrity.logBlind(context, profile, date, l, g, note)
                    savedHard = !g
                    com.kshana.app.data.Backup.autoSave(context)
                }
            }, modifier = Modifier.fillMaxWidth())
        } else {
            Text(tr("Saved, in your words"), style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
            Text(
                tr("It is in your journal now, and it helps tune your birth time. It never counts toward the score itself: only predictions sealed before their dates do."),
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
            if (result.isEmpty()) {
                Text(
                    tr("Nothing sealed covered that day and part of life. That is common: most days are not foretold, and the record stays honest either way."),
                    style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
                )
            } else {
                SectionLabel(tr("Sealed before it happened"))
                for (m in result) {
                    HorizontalDivider(color = KshanaColors.LineSoft)
                    Column(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        Text(m.words, fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 22.sp, color = KshanaColors.Text)
                        Text(
                            tr(if (m.agrees) "Went the way it was sealed." else "Went the other way."),
                            style = MaterialTheme.typography.bodySmall, color = if (m.agrees) KshanaColors.Leaf else KshanaColors.Sky,
                        )
                    }
                }
            }
            if (savedHard) {
                Column(Modifier.fillMaxWidth().glass(14.dp).padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text(tr("A hard time passes. Small steady things help: a kept routine, a kind act, a word with someone you trust."), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                    Text(tr("WHAT HELPS ›"), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onRemedy).padding(vertical = 4.dp))
                }
            }
            QuietButton(tr("WRITE ANOTHER"), onClick = { saved = null; note = ""; good = null }, modifier = Modifier.fillMaxWidth())
            QuietButton(tr("YOUR SCORE"), onClick = onScore, modifier = Modifier.fillMaxWidth())
        }
    }
}
