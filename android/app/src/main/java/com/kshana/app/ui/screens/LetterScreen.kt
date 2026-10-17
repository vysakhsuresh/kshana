package com.kshana.app.ui.screens

import android.content.Intent
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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
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
import uniffi.kshana_ffi.LetterRecord

internal object LetterCache {
    var key: String? = null
    var record: LetterRecord? = null
}

/** The month's letter: one page for the thirty days ahead, kept and shareable. */
@Composable
fun LetterScreen(profile: BirthProfile, place: Place, settings: EngineSettings, onBack: () -> Unit, onLife: (String) -> Unit, onWall: () -> Unit) {
    val context = LocalContext.current
    val key = "${profile.birthInstant().taiSeconds}|${place.name}|${settings.ayanamsa}|${java.time.LocalDate.now()}"
    var letter by remember { mutableStateOf(if (LetterCache.key == key) LetterCache.record else null) }
    var error by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(key) {
        if (letter != null) return@LaunchedEffect
        try {
            val engine = EngineProvider.get(context)
            val l = withContext(Dispatchers.Default) {
                engine.letter(profile.birthInstant(), profile.latitude, profile.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, Instants.now())
            }
            LetterCache.key = key
            LetterCache.record = l
            letter = l
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("THE LETTER", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        ErrorLine(error)
        val l = letter
        if (l == null) {
            Text("The month's letter", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
            if (error == null) Working("writing thirty days…")
            return@Column
        }
        Text(l.title, style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text("${profile.name} · birth Moon ${l.briefing.moon} · written ${localText(l.from, DATE_ONLY)} for ${place.name}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        Column(
            Modifier.fillMaxWidth().glass(18.dp).padding(20.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            Text("Dear ${profile.name.substringBefore(' ').lowercase().replaceFirstChar { it.uppercase() }},", fontFamily = FontFamily.Serif, fontStyle = FontStyle.Italic, fontSize = 18.sp, color = KshanaColors.TextSoft)
            for (p in l.paragraphs) Text(p, fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 26.sp, color = KshanaColors.Text)
            Text("Yours, to the second,\nKshana", fontFamily = FontFamily.Serif, fontStyle = FontStyle.Italic, fontSize = 16.sp, lineHeight = 22.sp, color = KshanaColors.TextSoft)
        }
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            QuietButton("SHARE THE LETTER", onClick = {
                val text = buildString {
                    appendLine(l.title)
                    appendLine()
                    for (p in l.paragraphs) { appendLine(p); appendLine() }
                    appendLine("Kshana · computed on the phone, to the second.")
                }
                val i = Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_SUBJECT, l.title).putExtra(Intent.EXTRA_TEXT, text)
                runCatching { context.startActivity(Intent.createChooser(i, "Share the letter")) }
            }, modifier = Modifier.weight(1f))
            QuietButton("THE WALL", onClick = onWall, modifier = Modifier.weight(1f))
        }

        SectionLabel("Each life, over the month")
        for (s in l.stories) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            Row(Modifier.fillMaxWidth().clickable { onLife(s.life) }.padding(vertical = 8.dp), verticalAlignment = Alignment.Top) {
                Text(lifeGlyph(s.life), fontSize = 16.sp, color = scoreColor(s.mean), modifier = Modifier.width(26.dp))
                Column(Modifier.weight(1f)) {
                    Text("${s.life} · ${s.trend}", style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text)
                    Text(s.trendWords, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                    val bs = s.bestStart
                    if (bs != null) Text("best ${localText(bs, DAY_MON)} · ${s.bestScore.toInt()}" + (s.lowStart?.let { "  ·  go easy ${localText(it, DAY_MON)} · ${s.lowScore.toInt()}" } ?: ""), fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky)
                }
                Text(s.mean.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 18.sp, color = scoreColor(s.mean))
            }
        }
        if (l.festivals.isNotEmpty()) {
            SectionLabel("Observances")
            for (f in l.festivals) {
                Row(Modifier.fillMaxWidth().padding(vertical = 3.dp)) {
                    Text(localText(f.at, WEEKDAY_DAY), fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky, modifier = Modifier.width(96.dp))
                    Column(Modifier.weight(1f)) {
                        Text(f.name, style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text)
                        Text(f.what, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                    }
                }
            }
        }
        Spacer(Modifier.height(4.dp))
        Text("A fresh letter is written each day you open it; the one on your birth star's day is the month's.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    }
}
