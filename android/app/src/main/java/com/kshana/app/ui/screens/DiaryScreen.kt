package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.DiaryEntry
import com.kshana.app.data.DiaryStore
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.time.DateTimeException
import java.time.Instant
import java.time.YearMonth
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import com.kshana.app.ui.sky.KshanaFonts

private val MONTH_CHIP get() = com.kshana.app.ui.screens.datePattern("MMM uuuu")
private const val CONFIRM_WINDOW_MS = 4_000L
private const val SNAPSHOT_DEPTH: UByte = 3u

/**
 * The sky diary: a note field, and every note the person has kept, newest first, each
 * stamped with the exact local instant it was written and the sky running then (the dasha
 * chain, the Moon's sign and star, the rising sign, the tithi, the hora). The stamp is
 * taken at the press, before the engine is asked, so the note carries the instant of the
 * press and not of the computation.
 */
@Composable
fun DiaryScreen(profile: BirthProfile, here: Place, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val zone = remember(here, profile) {
        try { com.kshana.app.data.CityClocks.of(here.zoneId) } catch (e: DateTimeException) { profile.zone() }
    }
    var entries by remember(profile.name) { mutableStateOf(DiaryStore.entries(context, profile.name)) }
    var text by remember { mutableStateOf("") }
    var keeping by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var natalMoon by remember(profile, settings) { mutableStateOf<Double?>(null) }
    var month by remember { mutableStateOf<YearMonth?>(null) }
    var confirming by remember { mutableStateOf<String?>(null) }

    // The natal Moon seeds the dasha chain; read once from the birth chart.
    LaunchedEffect(profile, settings) {
        try {
            val engine = EngineProvider.get(context)
            natalMoon = withContext(Dispatchers.Default) {
                engine.chart(profile.birthInstant(), profile.latitude, profile.longitude, 0.0, settings.houseSystem, settings.ayanamsa, false)
                    .panchanga.moonSidereal
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    // A first tap on REMOVE waits four seconds for the second; then it is forgotten.
    LaunchedEffect(confirming) {
        if (confirming != null) {
            delay(CONFIRM_WINDOW_MS)
            confirming = null
        }
    }

    fun keep() {
        val body = text.trim()
        if (body.isEmpty() || keeping) return
        val at = Instants.now()
        val atMillis = System.currentTimeMillis()
        keeping = true
        scope.launch {
            var line = ""
            var problem: String? = null
            try {
                val engine = EngineProvider.get(context)
                line = withContext(Dispatchers.Default) {
                    val moon = natalMoon ?: engine.chart(
                        profile.birthInstant(), profile.latitude, profile.longitude, 0.0, settings.houseSystem, settings.ayanamsa, false,
                    ).panchanga.moonSidereal
                    DiaryStore.snapshotLine(
                        engine.snapshot(profile.birthInstant(), moon, here.latitude, here.longitude, settings.ayanamsa, at, SNAPSHOT_DEPTH, com.kshana.app.data.Lang.code),
                    )
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                problem = e.message ?: e.toString()
            }
            // The words are kept even when the sky is not computed; the stamp still holds the instant.
            entries = DiaryStore.add(context, profile.name, DiaryEntry(atMillis, at.taiSeconds, at.nanos, body, line))
            text = ""
            error = problem?.let { "The sky for this note is not computed: $it" }
            keeping = false
        }
    }

    val months = remember(entries, zone) {
        entries.map { YearMonth.from(Instant.ofEpochMilli(it.atMillis).atZone(zone)) }.distinct()
    }
    val chosen = month?.takeIf { it in months }
    val shown = remember(entries, chosen, zone) {
        if (chosen == null) entries
        else entries.filter { YearMonth.from(Instant.ofEpochMilli(it.atMillis).atZone(zone)) == chosen }
    }

    LazyColumn(
        Modifier.fillMaxSize(),
        contentPadding = PaddingValues(start = 24.dp, end = 24.dp, top = 48.dp, bottom = 40.dp),
    ) {
        item(key = "header") {
            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                    Text("THE DIARY · ${here.name.uppercase()}", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                    Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
                }
                Text("Under this sky", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
                Text(
                    "Each note keeps the instant you write it. The sky running at that instant is stamped beside it.",
                    style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
                )
                OutlinedTextField(
                    value = text,
                    onValueChange = { text = it },
                    placeholder = { Text("Write under this sky", fontFamily = KshanaFonts.serif, fontSize = 17.sp, color = KshanaColors.Muted) },
                    minLines = 3,
                    maxLines = 8,
                    textStyle = TextStyle(fontFamily = KshanaFonts.serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Text),
                    colors = diaryFieldColors(),
                    modifier = Modifier.fillMaxWidth(),
                )
                GoldButton(
                    if (keeping) "KEEPING" else "KEEP",
                    enabled = text.isNotBlank() && !keeping,
                    onClick = { keep() },
                    modifier = Modifier.fillMaxWidth(),
                )
                ErrorLine(error)
                if (months.size > 1) {
                    Row(
                        Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(top = 6.dp),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Chip("All", selected = chosen == null, onClick = { month = null })
                        for (m in months) {
                            Chip(m.format(MONTH_CHIP), selected = chosen == m, onClick = { month = m })
                        }
                    }
                }
                Spacer(Modifier.height(8.dp))
                HorizontalDivider(color = KshanaColors.LineSoft)
            }
        }
        if (shown.isEmpty()) {
            item(key = "empty") {
                Text(
                    if (entries.isEmpty()) "No notes yet. The first one is stamped with the sky at the instant you keep it."
                    else "No notes in this month.",
                    style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
                    modifier = Modifier.padding(vertical = 16.dp),
                )
            }
        }
        items(shown, key = { it.key }) { entry ->
            EntryRow(
                entry = entry,
                zone = zone,
                confirming = confirming == entry.key,
                onRemove = {
                    if (confirming == entry.key) {
                        entries = DiaryStore.remove(context, profile.name, entry)
                        confirming = null
                    } else {
                        confirming = entry.key
                    }
                },
            )
        }
    }
}

/** One note: the words in serif, the instant to the second in mono, the sky line in muted. */
@Composable
private fun EntryRow(entry: DiaryEntry, zone: ZoneId, confirming: Boolean, onRemove: () -> Unit) {
    Column(Modifier.fillMaxWidth().padding(top = 14.dp, bottom = 4.dp)) {
        Text(entry.text, fontFamily = KshanaFonts.serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Text)
        Text(
            Instant.ofEpochMilli(entry.atMillis).atZone(zone).format(DATE_TIME_S),
            fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky,
            modifier = Modifier.padding(top = 8.dp),
        )
        if (entry.snapshot.isNotEmpty()) {
            Text(entry.snapshot, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, lineHeight = 16.sp, modifier = Modifier.padding(top = 2.dp))
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
            Text(
                if (confirming) "TAP AGAIN TO REMOVE" else "REMOVE",
                style = MaterialTheme.typography.labelSmall,
                color = if (confirming) KshanaColors.Gold else KshanaColors.Muted,
                modifier = Modifier.clickable(onClick = onRemove).padding(horizontal = 12.dp, vertical = 12.dp),
            )
        }
    }
    HorizontalDivider(color = KshanaColors.LineSoft)
}

@Composable
private fun diaryFieldColors() = OutlinedTextFieldDefaults.colors(
    focusedTextColor = KshanaColors.Text,
    unfocusedTextColor = KshanaColors.Text,
    cursorColor = KshanaColors.Gold,
    focusedBorderColor = KshanaColors.Gold,
    unfocusedBorderColor = KshanaColors.Line,
    focusedPlaceholderColor = KshanaColors.Muted,
    unfocusedPlaceholderColor = KshanaColors.Muted,
    focusedContainerColor = KshanaColors.Surface,
    unfocusedContainerColor = KshanaColors.Surface,
)
