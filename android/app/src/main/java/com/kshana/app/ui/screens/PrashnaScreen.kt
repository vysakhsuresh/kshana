package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.PrashnaRecord
import uniffi.kshana_ffi.QuestionShape
import java.time.Instant
import java.time.ZoneId
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter
import com.kshana.app.ui.sky.KshanaFonts

/** Display label → engine area name. */
private val AREA_CHIPS = listOf(
    "Career" to "Career", "Marriage" to "Marriage", "Health" to "Health", "Money" to "Wealth",
    "Property" to "Property", "Travel" to "Foreign", "Children" to "Children", "Study" to "Education",
)

private val SHAPE_CHIPS = listOf(
    "Will it happen?" to QuestionShape.WILL_IT_HAPPEN,
    "When?" to QuestionShape.WHEN,
    "Which choice?" to QuestionShape.CHOICE,
    "Will it return?" to QuestionShape.WILL_IT_RETURN,
    "Will they recover?" to QuestionShape.WILL_THEY_RECOVER,
)

/** Keyword → engine area. Only these words are read; nothing else in the text is interpreted. */
private val KEYWORDS = listOf(
    listOf("job", "offer", "promotion") to "Career",
    listOf("marry", "marriage", "wedding") to "Marriage",
    listOf("child", "baby") to "Children",
    listOf("money", "loan", "salary") to "Wealth",
    listOf("house", "flat", "land") to "Property",
    listOf("abroad", "visa", "travel") to "Foreign",
    listOf("exam", "admission", "study") to "Education",
    listOf("health", "illness", "recover") to "Health",
)

private const val WINDOW_DAYS = 30u
private val CLOCK_DATE = DateTimeFormatter.ofPattern("EEEE, d MMMM uuuu")
private val CLOCK_TIME = DateTimeFormatter.ofPattern("HH:mm:ss")

private fun areaFromText(text: String): String? {
    val lower = text.lowercase()
    for ((words, area) in KEYWORDS) if (words.any { lower.contains(it) }) return area
    return null
}

/** Three taps and an instant: area, shape, optional KP number; then the answer card. */
@Composable
fun PrashnaScreen(place: Place, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var area by remember { mutableStateOf<String?>(null) }
    var shape by remember { mutableStateOf<QuestionShape?>(null) }
    var typed by remember { mutableStateOf("") }
    var number by remember { mutableStateOf("") }
    var tick by remember { mutableLongStateOf(System.currentTimeMillis()) }
    var casting by remember { mutableStateOf(false) }
    var askedAt by remember { mutableStateOf<ExactInstant?>(null) }
    var answer by remember { mutableStateOf<PrashnaRecord?>(null) }
    var error by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        while (true) {
            tick = System.currentTimeMillis()
            delay(31)
        }
    }

    val mapped = remember(typed) { areaFromText(typed) }
    LaunchedEffect(mapped) { if (mapped != null) area = mapped }
    val kp = number.toIntOrNull()
    val kpValid = number.isBlank() || (kp != null && kp in 1..249)
    val ready = area != null && shape != null && kpValid && !casting

    fun cast() {
        val a = area ?: return
        val s = shape ?: return
        casting = true
        error = null
        scope.launch {
            val at = Instants.now()
            try {
                val engine = EngineProvider.get(context)
                val rec = withContext(Dispatchers.Default) {
                    engine.prashna(at, place.latitude, place.longitude, kp?.toUShort(), a, s, settings.ayanamsa, WINDOW_DAYS)
                }
                askedAt = at
                answer = rec
                // A Yes that names a time is a dated prediction: sealed before its time opens.
                if (s != QuestionShape.CHOICE) com.kshana.app.data.Trust.sealPrashna(context, a, rec)
            } catch (e: Exception) {
                error = e.message ?: e.toString()
            }
            casting = false
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("PRASHNA", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        val rec = answer
        val at = askedAt
        if (rec == null || at == null) {
            Text("Ask the moment", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
            Text("${place.name} · ${"%.4f".format(place.latitude)}° ${"%.4f".format(place.longitude)}°", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            LiveClock(tick)

            SectionLabel("About")
            Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for ((label, name) in AREA_CHIPS) Chip(label.uppercase(), selected = area == name, onClick = { area = name })
            }
            SectionLabel("Shape")
            Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for ((label, s) in SHAPE_CHIPS) Chip(label.uppercase(), selected = shape == s, onClick = { shape = s })
            }

            SectionLabel("Or type it (optional)")
            OutlinedTextField(
                value = typed, onValueChange = { typed = it },
                label = { Text("A few words of the question", style = MaterialTheme.typography.labelSmall) },
                singleLine = true,
                textStyle = TextStyle(fontFamily = KshanaFonts.sans, fontSize = 15.sp, color = KshanaColors.Text),
                colors = prashnaFieldColors(), modifier = Modifier.fillMaxWidth(),
            )
            if (typed.isNotBlank()) {
                Text(
                    if (mapped != null) "Reads as: $mapped. Only the keyword is used; the words themselves are not interpreted."
                    else "No known keyword. Pick an area above.",
                    style = MaterialTheme.typography.bodySmall, color = if (mapped != null) KshanaColors.Sky else KshanaColors.Muted,
                )
            }

            SectionLabel("KP number (optional)")
            OutlinedTextField(
                value = number, onValueChange = { number = it.filter(Char::isDigit).take(3) },
                label = { Text("1–249, or blank for the lagna of the moment", style = MaterialTheme.typography.labelSmall) },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                textStyle = TextStyle(fontFamily = KshanaFonts.mono, fontSize = 15.sp, color = KshanaColors.Text),
                colors = prashnaFieldColors(), modifier = Modifier.fillMaxWidth(),
            )
            if (!kpValid) Text("The number must be between 1 and 249.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)

            Spacer(Modifier.height(8.dp))
            ErrorLine(error)
            GoldButton(if (casting) "CASTING…" else "CAST", enabled = ready, onClick = { cast() }, modifier = Modifier.fillMaxWidth())
            Text("The instant of the tap is the chart. Area: ${area ?: "—"} · shape: ${SHAPE_CHIPS.firstOrNull { it.second == shape }?.first ?: "—"}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        } else {
            AnswerCard(rec, at, area ?: "", place) {
                answer = null; askedAt = null; area = null; shape = null; typed = ""; number = ""
            }
        }
    }
}

@Composable
private fun LiveClock(tickMillis: Long) {
    val local = ZonedDateTime.ofInstant(Instant.ofEpochMilli(tickMillis), ZoneId.systemDefault())
    val micro = ((System.nanoTime() / 1_000) % 1_000_000).toString().padStart(6, '0')
    Column {
        Text(local.format(CLOCK_DATE), fontFamily = KshanaFonts.serif, fontSize = 20.sp, color = KshanaColors.TextSoft)
        Row(verticalAlignment = Alignment.Bottom) {
            Text(local.format(CLOCK_TIME), style = MaterialTheme.typography.headlineLarge, color = KshanaColors.Text)
            Text(".${micro.substring(0, 3)} ${micro.substring(3)}", fontFamily = KshanaFonts.mono, fontSize = 20.sp, color = KshanaColors.Gold, modifier = Modifier.padding(start = 6.dp, bottom = 6.dp))
        }
    }
}

@Composable
private fun AnswerCard(rec: PrashnaRecord, at: ExactInstant, area: String, place: Place, onReset: () -> Unit) {
    Text(area.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
    Text(rec.verdict, style = MaterialTheme.typography.displayLarge, color = KshanaColors.Gold)
    Text("asked ${localText(at, DATE_TIME_S)}", fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky)
    Text(microText(at), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Gold)
    if (!rec.questionGenuine) {
        Text("The chart does not mark this question as genuine; read the answer lightly.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Sky)
    }
    Text(
        "lagna ${degMin(rec.lagnaSidereal)} ${signName(rec.lagnaSidereal)} · Moon ${degMin(rec.moonSidereal)} ${signName(rec.moonSidereal)} · Moon sub ${rec.moonSubLord}",
        fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted,
    )

    SectionLabel("Why")
    val factors = rec.hits.flatMap { it.factors }.ifEmpty { rec.promise.factors }
    if (factors.isEmpty()) Text("no factors recorded", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
    for (f in factors) {
        HorizontalDivider(color = KshanaColors.LineSoft)
        Text(f, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.padding(vertical = 6.dp))
    }
    for (h in rec.hits) {
        Text("${h.ruleId} · ${h.source} · weight ${"%.2f".format(h.weight)}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
    }

    SectionLabel("Ruling planets")
    Text(rec.rulingPlanets.joinToString(" · ").ifEmpty { "—" }, fontFamily = KshanaFonts.serif, fontSize = 18.sp, color = KshanaColors.Text)
    SectionLabel("Agents")
    Text(rec.agents.joinToString(" · ").ifEmpty { "—" }, fontFamily = KshanaFonts.serif, fontSize = 18.sp, color = KshanaColors.Text)

    SectionLabel("Moon windows")
    if (rec.windows.isEmpty()) Text("none in the next $WINDOW_DAYS days", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
    for (w in rec.windows.take(6)) {
        HorizontalDivider(color = KshanaColors.LineSoft)
        Row(Modifier.fillMaxWidth().padding(vertical = 8.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text("${w.star} star · ${w.sub} sub", fontFamily = KshanaFonts.serif, fontSize = 16.sp, color = KshanaColors.Text)
                Text("${localText(w.start, DATE_TIME_S)} → ${localText(w.end, DATE_TIME_S)}", fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky)
            }
            Column(horizontalAlignment = Alignment.End) {
                Text(microText(w.start), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Gold)
                Text(microText(w.end), fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Gold)
            }
        }
    }
    Spacer(Modifier.height(8.dp))
    Text(
        "Prashna answers one sincere question about this moment from ${place.name}. It will not answer idle curiosity, a matter already settled, or the same question asked again.",
        style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
    )
    GoldButton("ASK ANOTHER", onClick = onReset, modifier = Modifier.fillMaxWidth().padding(top = 8.dp))
}

@Composable
private fun prashnaFieldColors() = OutlinedTextFieldDefaults.colors(
    focusedTextColor = KshanaColors.Text,
    unfocusedTextColor = KshanaColors.Text,
    cursorColor = KshanaColors.Gold,
    focusedBorderColor = KshanaColors.Gold,
    unfocusedBorderColor = KshanaColors.Line,
    focusedLabelColor = KshanaColors.Gold,
    unfocusedLabelColor = KshanaColors.Muted,
    focusedContainerColor = KshanaColors.Surface,
    unfocusedContainerColor = KshanaColors.Surface,
)
