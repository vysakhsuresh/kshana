package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Journal
import com.kshana.app.data.Lang
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.PastCheckRecord

/**
 * Before predicting anything, the app reads your past back to you: a few real
 * stretches of the last years, as yes-or-no questions. You see at once whether it can
 * see your life. Every "yes" goes into your journal and sharpens the readings.
 */
@Composable
fun PastCheckScreen(profile: BirthProfile, settings: EngineSettings, onDone: () -> Unit) {
    val context = LocalContext.current
    var checks by remember { mutableStateOf<List<PastCheckRecord>?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    val answers = remember { mutableStateMapOf<Int, Boolean>() }
    LaunchedEffect(profile) {
        try {
            val engine = EngineProvider.get(context)
            checks = withContext(Dispatchers.Default) {
                engine.pastChecks(profile.birthInstant(), profile.latitude, profile.longitude, settings.houseSystem, settings.ayanamsa, Instants.now(), Lang.code)
            }
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 56.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("FIRST, YOUR PAST", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text("Did these happen?", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "Before it tells you anything about the future, Kshana reads a few stretches of your last years from your birth time alone. Answer honestly; you will see how well it knows your life.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        ErrorLine(error)
        val list = checks
        if (list == null) {
            if (error == null) Working("reading your last years…")
            return@Column
        }
        for ((i, c) in list.withIndex()) {
            val a = answers[i]
            Column(Modifier.fillMaxWidth().glass(16.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("${lifeGlyph(c.life)} ${lifeName(c.life).uppercase()}", style = MaterialTheme.typography.labelSmall, color = if (c.good) KshanaColors.Leaf else KshanaColors.Ember)
                Text(c.question, fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Text)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OptionBox("Yes", selected = a == true, onClick = {
                        if (answers[i] != true) {
                            answers[i] = true
                            val mid = Instants.toLocal(c.from).toLocalDate().plusDays(java.time.temporal.ChronoUnit.DAYS.between(Instants.toLocal(c.from).toLocalDate(), Instants.toLocal(c.to).toLocalDate()) / 2)
                            Journal.add(context, mid, c.life, c.good, "Confirmed at the first check")
                        }
                    }, modifier = Modifier.weight(1f))
                    OptionBox("No", selected = a == false, onClick = { answers[i] = false }, modifier = Modifier.weight(1f))
                    OptionBox("Not sure", selected = false, onClick = { answers.remove(i) }, modifier = Modifier.weight(1f))
                }
            }
        }
        val yes = answers.values.count { it }
        val answered = answers.size
        if (answered > 0) {
            Text(
                if (answered == list.size) "It read $yes of ${list.size} stretches of your past correctly. That is the honest starting point; the record keeps growing as you use it."
                else "$yes of $answered so far.",
                fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Gold,
            )
            if (answered == list.size && yes * 2 < list.size) {
                Text(
                    "Fewer than half matched. The birth time may be off: try More › Find your exact birth time. It uses what you just answered.",
                    style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
                )
            }
        }
        GoldButton(if (answered == 0) "SKIP FOR NOW" else "SEE MY DAY", onClick = onDone, modifier = Modifier.fillMaxWidth())
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.Center, verticalAlignment = Alignment.CenterVertically) {
            Text("Your answers stay on this phone.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onDone))
        }
    }
}
