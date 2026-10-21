package com.kshana.app.ui.screens

import androidx.compose.animation.AnimatedVisibility
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
import com.kshana.app.ui.theme.AnimatedTabIcon
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.TabGlyph
import com.kshana.app.ui.theme.glass

private data class GuideStop(val glyph: TabGlyph?, val title: String, val body: String, val route: String? = null)

private val TABS_GUIDE = listOf(
    GuideStop(TabGlyph.SUN, "Now", "Today, as it is. The big number is how the day is going for you from 0 to 100, with when it changes next and what the hour is good for. Below it, each part of your life (love, work, money, health and more) has its own score. Use TRAVEL to see another hour or day, and READ IT ALOUD to hear it.", "loop"),
    GuideStop(TabGlyph.EYE, "Reading", "Your own consultation. Say what is on your mind in your own words, or pick a matter, and get an answer with dates: when it opens, when to wait, and what helps. If the app does not understand a question it says so and asks you to pick, rather than guess.", "reading"),
    GuideStop(TabGlyph.CRYSTAL, "Future", "What is coming, in order. DAY TO YEAR shows today, this week, this month and this year. YEARS AHEAD lists the openings and hard stretches over 1, 3 or 5 years, each with its months, how strong it is, and what to do. Tap CHANGE to choose which parts of life you care about.", "future"),
    GuideStop(TabGlyph.HEART, "Us", "Two people together: add someone close to you and see how your times fit, the good days to talk, and where to go gently with each other.", "together"),
    GuideStop(TabGlyph.STAR, "Favorites", "Your own shelf. Tap the ☆ beside any page under Explore and it stays here, one tap away. Pages you open often show up here too.", "favorites"),
    GuideStop(TabGlyph.MORE, "Explore", "Every other page, grouped by what you need: something weighs on me, a day to plan, waiting for something, my life so far, my family. A gold dot marks pages you have not opened yet. Your birth details, settings and help are here too, and (if you switch them on in Settings) the chart pages for people who read charts.", "more"),
)

private val TIPS = listOf(
    "Reading the numbers" to "Above 60 is a good stretch, 40 to 60 is steady, below 40 asks for care. The words under each number tell you what it means for you; you never need to know how it is worked out.",
    "Exact birth time" to "The closer your birth time, the sharper every date. Explore › Your past, read tells you dated stretches of your own life; say Yes, Close or No and it works out the birth time that fits your life best. A wrong guess still helps, because it narrows down the birth time. Ask for more questions as often as you like.",
    "The sealed prediction" to "After reading your past, Kshana writes down one thing about the coming weeks and seals it. It opens on its date, on the Now screen, and you say whether it happened. The seal code proves the words were not changed.",
    "Remedies" to "When something weighs on you, Explore › What helps gives simple steps anyone can take, with prayers of your own faith if you want them. Nothing is ever required.",
    "Reminders" to "Tap REMIND ME on any date to get a notice before it begins, or CALENDAR to put it in your phone's calendar.",
    "Did it happen?" to "Tell the app whether its readings came true. It keeps an open score of how often it was right for you, and it learns which parts of your life it reads best.",
    "Your data" to "Everything stays on this phone. A copy is saved in Downloads › Kshana, so if you change phones or reinstall, choose RESTORE FROM MY BACKUP FILE on the first screen.",
    "Language" to "More › Settings lets you read the app in English, Malayalam or Hindi.",
)

/** How to use Kshana: each tab with its moving icon, then the few things worth knowing. */
@Composable
fun GuideScreen(onBack: () -> Unit, onOpen: (String) -> Unit) {
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("GUIDE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("How to use Kshana", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "Six tabs along the bottom. Each one answers a different question about your time. Tap a card to go there.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
        )

        SectionLabel("The six tabs")
        for (s in TABS_GUIDE) {
            Row(
                Modifier.fillMaxWidth().glass(14.dp).clickable { s.route?.let(onOpen) }.padding(16.dp),
                horizontalArrangement = Arrangement.spacedBy(14.dp),
            ) {
                if (s.glyph != null) {
                    androidx.compose.runtime.CompositionLocalProvider(androidx.compose.material3.LocalContentColor provides KshanaColors.Gold) {
                        AnimatedTabIcon(s.glyph, selected = true, label = s.title)
                    }
                }
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(s.title.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                    Text(s.body, fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 24.sp, color = KshanaColors.Text)
                }
            }
        }

        SectionLabel("Good to know")
        for ((title, body) in TIPS) Expandable(title, body)

        QuietButton("HOW KSHANA WORKS, IN DEPTH", onClick = { onOpen("how") }, modifier = Modifier.fillMaxWidth().padding(top = 8.dp))
    }
}

@Composable
private fun Expandable(title: String, body: String) {
    var open by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth().glass(14.dp).clickable { open = !open }.padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(title.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text(if (open) "–" else "+", color = KshanaColors.Gold)
        }
        AnimatedVisibility(open) {
            Text(body, fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 24.sp, color = KshanaColors.Text)
        }
    }
}

/** Who made Kshana, what the name means, what it stands on, and the version. */
@Composable
fun AboutScreen(onBack: () -> Unit, onOpen: (String) -> Unit) {
    val context = LocalContext.current
    val version = remember {
        runCatching {
            val pi = context.packageManager.getPackageInfo(context.packageName, 0)
            "${pi.versionName} (${androidx.core.content.pm.PackageInfoCompat.getLongVersionCode(pi)})"
        }.getOrDefault("")
    }
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("ABOUT", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("K S H A N A", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Gold)
        Text("क्षण · the moment", fontFamily = FontFamily.Serif, fontSize = 18.sp, color = KshanaColors.TextSoft)

        AboutCard("The name", "Kshana is the Sanskrit word for a moment: the smallest piece of time that still matters. The app is built on one idea: that every moment of your life has its own weather, and that knowing it ahead helps you meet it well.")
        AboutCard("Why we made it", "Most astrology apps speak a language only astrologers understand, or say the same thing to everyone. Kshana works from your own birth and the real sky, and tells you in plain words what is coming, when, how strongly, and what to do. Guidance, never a guarantee, and it shows you openly how often it has been right.")
        RecordForYouCard(onOpen)
        AboutCard("What it stands on", "The positions of the sun, moon and planets come from NASA's planetary data and the international Earth-rotation service, worked out on this phone to a fraction of a second. The readings follow long-tested rules from the classic Indian texts, weighed together and checked against your own life.")
        AboutCard("Places", "Every town and village in India and every town of 1,000 people or more worldwide, with their own coordinates and time zones, from GeoNames (geonames.org), used under the Creative Commons Attribution 4.0 licence.")
        AboutCard("Your privacy", "No account, no sign-in, no advertising, no tracking. Nothing you enter leaves this phone unless you send it yourself.")
        AboutCard("Made by", "${com.kshana.app.data.BrandLinks.BRAND_LABEL}, with care.")

        Column(Modifier.fillMaxWidth().glass(14.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Text("TALK TO US", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text(
                "A question, something that went wrong, or a reading that did not fit? Tell us. We read every message.",
                fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 24.sp, color = KshanaColors.Text,
            )
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                QuietButton("EMAIL US", onClick = {
                    com.kshana.app.data.BrandLinks.sendEmail(context, "Kshana $version - feedback")
                }, modifier = Modifier.weight(1f))
                QuietButton("WHATSAPP", onClick = {
                    com.kshana.app.data.BrandLinks.openUrl(context, com.kshana.app.data.BrandLinks.WHATSAPP_URL)
                }, modifier = Modifier.weight(1f))
            }
            Text(
                "Either one opens your own mail or WhatsApp app. Nothing from your readings is sent with it.",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
            )
        }

        Text(
            "Made by ${com.kshana.app.data.BrandLinks.BRAND_LABEL} · layerbit.co.in" + if (version.isNotBlank()) "\nVersion $version" else "",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
            modifier = Modifier.clickable { com.kshana.app.data.BrandLinks.openUrl(context, com.kshana.app.data.BrandLinks.WEBSITE_URL) }.padding(top = 4.dp, bottom = 4.dp),
        )
        QuietButton("GUIDE: HOW TO USE KSHANA", onClick = { onOpen("guide") }, modifier = Modifier.fillMaxWidth())
        QuietButton("HOW IT WORKS AND THE PRIVACY POLICY", onClick = { onOpen("how") }, modifier = Modifier.fillMaxWidth())
    }
}

/**
 * Kshana's record for you: the headline score (sealed predictions only, answered after
 * their dates) against guessing, how many are sealed and still to come, and a door to
 * the scoreboard. Before five are answered it says only that it is too early to tell.
 */
@Composable
private fun RecordForYouCard(onOpen: (String) -> Unit) {
    val context = LocalContext.current
    val profile = remember { com.kshana.app.data.ProfileStore.load(context) } ?: return
    val card = remember(profile.name) { com.kshana.app.data.Integrity.record(context, profile) } ?: return
    Column(Modifier.fillMaxWidth().glass(14.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text("KSHANA'S RECORD FOR YOU", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(card.headline, fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 26.sp, color = if (card.early) KshanaColors.Text else KshanaColors.Gold)
        Text(card.detail, style = MaterialTheme.typography.bodySmall, color = KshanaColors.TextSoft)
        Text("THE WHOLE SCOREBOARD ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable { onOpen("scoreboard") }.padding(vertical = 4.dp))
    }
}

@Composable
private fun AboutCard(title: String, body: String) {
    Column(Modifier.fillMaxWidth().glass(14.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text(title.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(body, fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 24.sp, color = KshanaColors.Text)
    }
}
