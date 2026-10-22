package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.FamilyStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.net.Narration
import com.kshana.app.net.narrate
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.SectionRecord
import com.kshana.app.ui.sky.KshanaFonts

/** What one narration was grounded in, for the footer line. */
private data class Grounding(val sentences: Int, val rules: Int)

/**
 * Optional narration: the engine's computed reading, sent with the user's own key to an online
 * model that is told to add nothing. The screen says exactly what leaves the phone before the
 * user taps Narrate, and keeps the last narration for as long as the screen lives.
 */
@Composable
fun NarrateScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit, onSettings: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val onlineOn = remember { FamilyStore.onlineAllowed(context) }
    val apiKey = remember { FamilyStore.aiKey(context) }
    val ready = onlineOn && apiKey != null

    var model by remember { mutableStateOf(Narration.DEFAULT_MODEL) }
    var question by remember { mutableStateOf("") }
    var writing by remember { mutableStateOf(false) }
    var narration by remember { mutableStateOf<String?>(null) }
    var grounding by remember { mutableStateOf<Grounding?>(null) }
    var error by remember { mutableStateOf<String?>(null) }

    fun run() {
        if (writing || apiKey == null) return
        scope.launch {
            writing = true
            error = null
            try {
                val engine = EngineProvider.get(context)
                val at = Instants.now()
                val (report, sections) = withContext(Dispatchers.Default) {
                    val r = engine.readingReportJson(profile.birthInstant(), profile.latitude, profile.longitude, settings.houseSystem, settings.ayanamsa, at)
                    val s: List<SectionRecord> = engine.plainReading(profile.birthInstant(), profile.latitude, profile.longitude, settings.ayanamsa, at, com.kshana.app.data.Lang.code)
                    Pair(r, s)
                }
                val text = narrate(context, apiKey, model, report, sections, question.trim().ifEmpty { null })
                narration = text
                grounding = Grounding(
                    sentences = sections.sumOf { it.sentences.size },
                    rules = sections.flatMap { s -> s.sentences.flatMap { it.rules } }.toSet().size,
                )
            } catch (e: Exception) {
                error = e.message ?: e.toString()
            }
            writing = false
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 40.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("NARRATE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("The reading, in words", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)

        SectionLabel("What leaves the phone")
        Text("Only the computed reading: the sentences, their confidence and reasons, and the chart report with the rules that fired.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        Text("No name, and no exact birth time beyond what the reading itself contains.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        Text("The words come from an online model, grounded in the engine's facts and told to add nothing of its own.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)

        if (!ready) {
            Spacer(Modifier.height(4.dp))
            Text(
                if (!onlineOn) "Online features are off." else "No narration key has been entered.",
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
            QuietButton("SET UP IN SETTINGS", onClick = onSettings)
        } else {
            SectionLabel("Model")
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for ((label, id) in Narration.MODELS) Chip(label.uppercase(), selected = model == id, onClick = { model = id })
            }
            Text(model, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)

            SectionLabel("Focus")
            OutlinedTextField(
                value = question,
                onValueChange = { question = it },
                label = { Text("Anything you want the narration to focus on?", style = MaterialTheme.typography.labelSmall) },
                minLines = 2,
                textStyle = TextStyle(fontFamily = KshanaFonts.sans, fontSize = 15.sp, color = KshanaColors.Text),
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

            Spacer(Modifier.height(4.dp))
            if (writing) {
                Text("Writing…", style = MaterialTheme.typography.titleLarge, color = KshanaColors.Gold)
            } else {
                GoldButton("NARRATE", onClick = { run() }, modifier = Modifier.fillMaxWidth())
            }
            ErrorLine(error)
        }

        val text = narration
        if (text != null) {
            SectionLabel("Narration")
            Column(
                Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(8.dp))
                    .background(KshanaColors.Surface)
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                for (para in text.split(Regex("\\n\\s*\\n")).map { it.trim() }.filter { it.isNotEmpty() }) {
                    Text(para, fontFamily = KshanaFonts.serif, fontSize = 17.sp, lineHeight = 26.sp, color = KshanaColors.Text)
                }
                HorizontalDivider(color = KshanaColors.LineSoft)
                val g = grounding
                if (g != null) {
                    Text(
                        "Grounded in ${g.sentences} sentences and ${g.rules} rules from the engine",
                        style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
                    )
                }
            }
        }
    }
}
