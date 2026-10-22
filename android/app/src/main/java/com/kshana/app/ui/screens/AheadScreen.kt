package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import uniffi.kshana_ffi.OutlookRecord

internal object OutlookCache {
    val records = HashMap<String, OutlookRecord>()
}

private val SPANS = listOf("day" to "TODAY", "week" to "WEEK", "month" to "MONTH", "year" to "YEAR")

/**
 * Ahead: your life today, this week, this month, this year, in plain words. Each life
 * says what the stretch holds, the day to use and the day to go easy, and what to do.
 * The astrology behind each line sits under one tap, never in the sentence.
 */
@Composable
fun AheadScreen(
    profile: BirthProfile,
    place: Place,
    settings: EngineSettings,
    onLife: (String) -> Unit,
    onRemedy: (String) -> Unit,
    onLetter: () -> Unit,
    onBook: () -> Unit,
    onHindsight: () -> Unit,
) {
    val context = LocalContext.current
    var span by remember { mutableStateOf(AheadRequest.span?.also { AheadRequest.span = null } ?: "day") }
    var out by remember { mutableStateOf<OutlookRecord?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var whyOpen by remember { mutableStateOf<String?>(null) }
    val hourKey = System.currentTimeMillis() / 3_600_000L
    val key = "${profile.birthInstant().taiSeconds}|${place.name}|${settings.ayanamsa}|$span|${if (span == "day") hourKey else hourKey / 24}|${com.kshana.app.data.Focus.lives}|${com.kshana.app.data.Lang.code}"

    LaunchedEffect(key) {
        out = OutlookCache.records[key]
        if (out != null) return@LaunchedEffect
        try {
            val engine = EngineProvider.get(context)
            val o = withContext(Dispatchers.Default) {
                engine.outlook(profile.birthInstant(), profile.latitude, profile.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, span, Instants.now(), com.kshana.app.data.Lang.code, com.kshana.app.data.Focus.lives)
            }
            OutlookCache.records[key] = o
            out = o
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("AHEAD", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(
            when (span) {
                "day" -> "Your day"
                "week" -> "Your week"
                "month" -> "Your month"
                else -> "Your year"
            },
            style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text,
        )
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((k, label) in SPANS) Chip(label, selected = span == k, onClick = { span = k; whyOpen = null }, modifier = Modifier.weight(1f))
        }
        ErrorLine(error)
        val o = out
        if (o == null) {
            if (error == null) Text(
                when (span) {
                    "day" -> "reading today…"
                    "week" -> "reading seven days…"
                    "month" -> "reading thirty days…"
                    else -> "reading the year, month by month…"
                },
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
            return@Column
        }

        // The whole, in a card: the ring, the dates, the headline, the season.
        Column(
            Modifier.fillMaxWidth().glass(18.dp).padding(18.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                ScoreRing(o.score, size = 84.dp, stroke = 7.dp, label = bandWord(o.band))
                Column(Modifier.weight(1f)) {
                    Text(
                        if (span == "day") localText(o.from, com.kshana.app.ui.screens.datePattern("EEEE d MMMM uuuu")) else "${localText(o.from, DATE_ONLY)} – ${localText(o.to, DATE_ONLY)}",
                        fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky,
                    )
                    Text(o.headline, fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Text)
                }
            }
            Text(o.season, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
            Row(horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                Text("SHARE ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable {
                    com.kshana.app.system.ShareCard.share(
                        context,
                        when (span) { "day" -> "My day"; "week" -> "My week"; "month" -> "My month"; else -> "My year" },
                        o.headline,
                        if (span == "day") localText(o.from, DATE_ONLY) else "${localText(o.from, DATE_ONLY)} – ${localText(o.to, DATE_ONLY)}",
                    )
                }.padding(vertical = 4.dp))
                Text("READ IT ALOUD ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable {
                    com.kshana.app.system.Speak.say(context, o.headline + " " + o.lives.take(3).joinToString(" ") { it.words }, com.kshana.app.data.Lang.code)
                }.padding(vertical = 4.dp))
            }
        }

        // Each life: the sentence, the dates, what to do.
        // Your chosen parts of life first; the rest below, apart, never in the summary.
        val ordered = o.lives.sortedBy { if (com.kshana.app.data.Focus.has(it.life)) 0 else 1 }
        var alsoShown = false
        for (l in ordered) {
            if (!com.kshana.app.data.Focus.has(l.life) && !alsoShown) {
                alsoShown = true
                SectionLabel("The rest of your life")
                Text("Parts you did not choose as yours. They are read too, but kept out of the summary above. Change your choice in Settings › What matters to me.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            }
            HorizontalDivider(color = KshanaColors.LineSoft)
            Column(Modifier.fillMaxWidth().clickable { onLife(l.life) }.padding(vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                    Text(lifeGlyph(l.life), fontSize = 16.sp, color = bandColor(l.band), modifier = Modifier.width(26.dp))
                    Text(lifeName(l.life), style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text, modifier = Modifier.weight(1f))
                    Text(bandWord(l.band), style = MaterialTheme.typography.labelSmall, color = bandColor(l.band))
                    Spacer(Modifier.width(10.dp))
                    Text(l.score.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 18.sp, color = scoreColor(l.score))
                }
                Text(l.words, style = MaterialTheme.typography.bodyLarge, color = KshanaColors.Text, modifier = Modifier.padding(start = 26.dp))
                Text(l.advice, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(start = 26.dp))
                Row(Modifier.padding(start = 26.dp), horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                    if (l.band == "slow" || l.band == "hold") {
                        Text("WHAT TO DO", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable { onRemedy(l.life) }.padding(vertical = 4.dp))
                    }
                    Text("MORE ABOUT ${lifeName(l.life).uppercase()} ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable { onLife(l.life) }.padding(vertical = 4.dp))
                }
            }
        }

        SectionLabel("More of your time")
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            QuietButton("THE MONTH'S LETTER", onClick = onLetter, modifier = Modifier.weight(1f))
            QuietButton("LOOK BACK AT A DAY", onClick = onHindsight, modifier = Modifier.weight(1f))
        }
        QuietButton("THE BOOK OF YOUR LIFE · THE YEARS AHEAD AND BEHIND", onClick = onBook, modifier = Modifier.fillMaxWidth())
    }
}

/** The span Ahead should open on, when a question asked for one. */
internal object AheadRequest {
    var span: String? = null
}
