package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
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
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.LanguageStore
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.LanguageRecord
import uniffi.kshana_ffi.LexiconEntryRecord
import uniffi.kshana_ffi.languageNames
import uniffi.kshana_ffi.lexicon
import uniffi.kshana_ffi.nazhika
import uniffi.kshana_ffi.vakyaNote
import java.time.DateTimeException
import java.time.ZoneId

/** The lexicon's families in display order, with the heading each gets. */
private val FAMILY_HEADINGS: List<Pair<String, String>> = listOf(
    "rasi" to "Rasi",
    "nakshatra" to "Nakshatra",
    "tithi" to "Tithi",
    "vara" to "Vara",
    "yoga" to "Yoga",
    "karana" to "Karana",
    "graha" to "Graha",
    "bhava" to "Bhava",
    "period" to "Periods",
    "clock" to "Day clocks",
    "observance" to "Observances",
    "month_kolla" to "Months · Kollavarsham",
    "month_tamil" to "Months · Tamil solar",
    "month_lunar" to "Months · lunar (Saka, Vikrama)",
)

/**
 * Every name the app uses, in the user's script: a language chip row (the choice is kept in
 * [LanguageStore] and used by every other screen through `t(context, key)`), a searchable list
 * of all terms with English, native script and transliteration side by side, the udayadi
 * nazhika for the current place right now, and the note on Vakya versus Drik panchanga.
 */
@Composable
fun LexiconScreen(onBack: () -> Unit) {
    val context = LocalContext.current
    var code by remember { mutableStateOf(LanguageStore.code(context)) }
    var languages by remember { mutableStateOf<List<LanguageRecord>>(emptyList()) }
    var entries by remember { mutableStateOf<List<LexiconEntryRecord>>(emptyList()) }
    var note by remember { mutableStateOf("") }
    var query by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var nazhikaLine by remember { mutableStateOf("Right now: reading the sunrise…") }

    LaunchedEffect(code) {
        try {
            val loaded = withContext(Dispatchers.Default) {
                Triple(languageNames(), lexicon(code), vakyaNote())
            }
            languages = loaded.first
            entries = loaded.second
            note = loaded.third
            error = null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    LaunchedEffect(Unit) {
        nazhikaLine = try {
            val engine = EngineProvider.get(context)
            val here = ProfileStore.currentLocation(context)
            val settings = ProfileStore.settings(context)
            withContext(Dispatchers.Default) {
                val now = Instants.now()
                val rise = engine.panchangaDetail(now, here.latitude, here.longitude, settings.ayanamsa).sunrise
                if (rise == null) {
                    "Right now: the Sun does not rise today at ${here.name}, so there is no udayadi count."
                } else {
                    val zone = try { ZoneId.of(here.zoneId) } catch (e: DateTimeException) { ZoneId.systemDefault() }
                    val r = nazhika(rise, now)
                    "Right now: nazhika ${r.nazhika}, vinazhika ${r.vinazhika} since sunrise " +
                        "(${localText(rise, TIME_ONLY, zone)} at ${here.name})."
                }
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            "Right now: no sunrise yet (${e.message ?: "engine unavailable"})."
        }
    }

    val shown = remember(entries, query) {
        val q = query.trim()
        if (q.isEmpty()) entries else entries.filter { e ->
            e.english.contains(q, ignoreCase = true) || e.key.contains(q, ignoreCase = true) ||
                e.sanskrit.contains(q, ignoreCase = true) || e.native.contains(q, ignoreCase = true) ||
                e.latin.contains(q, ignoreCase = true)
        }
    }

    Column(Modifier.fillMaxSize().padding(top = 48.dp)) {
        Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text("LEXICON", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
            }
            Text("Every name, in your script", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
            Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (l in languages) {
                    Chip(l.native, selected = code == l.code, onClick = {
                        LanguageStore.save(context, l.code)
                        code = l.code
                    })
                }
            }
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                label = { Text("Search in English, script or transliteration", style = MaterialTheme.typography.labelSmall) },
                singleLine = true,
                textStyle = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 15.sp, color = KshanaColors.Text),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedTextColor = KshanaColors.Text,
                    unfocusedTextColor = KshanaColors.Text,
                    cursorColor = KshanaColors.Gold,
                    focusedBorderColor = KshanaColors.Gold,
                    unfocusedBorderColor = KshanaColors.Line,
                    focusedLabelColor = KshanaColors.Gold,
                    unfocusedLabelColor = KshanaColors.Muted,
                    focusedContainerColor = KshanaColors.Surface,
                    unfocusedContainerColor = KshanaColors.Surface,
                ),
                modifier = Modifier.fillMaxWidth(),
            )
            ErrorLine(error)
            Text(
                if (entries.isEmpty() && error == null) "loading…" else "${shown.size} of ${entries.size} terms",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
            )
        }
        LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(start = 24.dp, end = 24.dp, top = 6.dp, bottom = 32.dp)) {
            item(key = "nazhika") {
                Text(nazhikaLine, fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky, modifier = Modifier.padding(vertical = 8.dp))
                Text(
                    "A dash means the word is not yet verified in this language; the English name is shown instead.",
                    style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
                )
            }
            for ((family, heading) in FAMILY_HEADINGS) {
                val rows = shown.filter { it.family == family }
                if (rows.isEmpty()) continue
                item(key = "head|$family") { SectionLabel(heading) }
                items(rows, key = { it.family + "|" + it.key }) { e -> TermRow(e) }
            }
            if (shown.isEmpty() && entries.isNotEmpty()) {
                item(key = "none") {
                    Text("Nothing matches. Try a shorter word.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(vertical = 12.dp))
                }
            }
            if (note.isNotEmpty()) {
                item(key = "vakya") {
                    SectionLabel("Vakya or Drik")
                    Text(note, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.padding(bottom = 12.dp))
                }
            }
        }
    }
}

/** English (with the Sanskrit or alternate name), native script and transliteration, side by side. */
@Composable
private fun TermRow(e: LexiconEntryRecord) {
    val verified = e.native.isNotBlank()
    HorizontalDivider(color = KshanaColors.LineSoft)
    Row(Modifier.fillMaxWidth().padding(vertical = 8.dp), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(e.english, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
            if (e.sanskrit.isNotBlank() && e.sanskrit != e.english) {
                Text(e.sanskrit, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
            }
        }
        Column(Modifier.weight(1f), horizontalAlignment = Alignment.End, verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(
                if (verified) e.native else "—",
                style = MaterialTheme.typography.titleLarge,
                color = if (verified) KshanaColors.Gold else KshanaColors.Muted,
            )
            Text(
                if (verified) e.latin else e.english,
                fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Muted,
            )
        }
    }
}
