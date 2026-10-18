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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass

private val HOW = listOf(
    "What it reads" to "Two things only: the moment you were born (date, time, place) and the real sky at every moment since. The positions of the sun, moon and planets are worked out on this phone from NASA's own data, to a fraction of a second, and checked against three independent references.",
    "How a reading is made" to "For each part of your life (love, work, money, health, family, study, home, travel, children) many long-tested rules of Indian astrology are weighed together into a score from 0 to 100. The long chapters of your life count most, then the season, then the day, then the hour. The sentences you read are written from those scores and their dates; the app never invents a line that the numbers do not support.",
    "Why it speaks plainly" to "The working uses the full language of astrology. You read only what it means for your life: what will happen, when, how strongly, and what to do. If you want the charts themselves, switch on 'Show the chart pages' in Settings.",
    "How sure it is" to "Every answer says how sure it is. The app keeps an open record of how often it was right for you: on the Now screen, in Did it happen?, and in your journal. Wrong ones are shown, never hidden.",
    "What it cannot do" to "It shows the weather of a season; it cannot promise an outcome. It is not a doctor, lawyer, counsellor or financial adviser, and it should never stop you from seeing one. Your own effort and choices always count.",
    "Remedies" to "Every remedy begins with deeds anyone can do, of any faith or none: how you treat people, what you give, your habits, care for nature, quiet for the mind. Prayer in your own faith and the Hindu tradition (poojas, mantras, temples) are offered as you choose in Settings. None is ever required.",
    "Your data" to "No account, no sign-in, no internet needed for any reading. Everything you enter stays on this phone, with a copy in Downloads › Kshana so it is never lost. It leaves the phone only if you send it yourself.",
)

/** How Kshana works, in plain words, with the privacy policy one tap below. */
@Composable
fun HowScreen(onBack: () -> Unit) {
    val context = LocalContext.current
    var showPolicy by remember { mutableStateOf(false) }
    val policy = remember { runCatching { context.assets.open("privacy.txt").bufferedReader().readText() }.getOrNull() }
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("OPEN BOOK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("How Kshana works", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        for ((title, body) in HOW) {
            Column(Modifier.fillMaxWidth().glass(14.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text(title.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text(body, fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 24.sp, color = KshanaColors.Text)
            }
        }
        if (policy != null) {
            QuietButton(if (showPolicy) "HIDE THE PRIVACY POLICY" else "READ THE PRIVACY POLICY", onClick = { showPolicy = !showPolicy }, modifier = Modifier.fillMaxWidth())
            if (showPolicy) Text(policy, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        }
    }
}
