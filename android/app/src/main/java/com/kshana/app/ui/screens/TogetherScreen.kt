package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.CoupleAnswerRecord
import uniffi.kshana_ffi.CoupleRecord
import uniffi.kshana_ffi.KundaliRecord
import uniffi.kshana_ffi.QuestionRecord
import uniffi.kshana_ffi.ReliefRecord
import uniffi.kshana_ffi.questions

/**
 * Us: the other person's birth beside yours. The classical match (36 points), Kuja
 * dosha, how the two of you stand at this moment, the months behind and ahead, what
 * lifts each side next, and any question about the two of you, answered from both charts.
 */
@Composable
fun TogetherScreen(
    profile: BirthProfile,
    place: Place,
    settings: EngineSettings,
    onAddPartner: () -> Unit,
    onFamilyDates: () -> Unit = {},
    onRemedy: (String) -> Unit = {},
) {
    val context = LocalContext.current
    var partners by remember { mutableStateOf(ProfileStore.partners(context)) }
    var pasteNote by remember { mutableStateOf<String?>(null) }
    var selected by remember { mutableIntStateOf(ProfileStore.selectedPartner(context).coerceIn(0, (partners.size - 1).coerceAtLeast(0))) }
    var selfIsGroom by remember { mutableStateOf(ProfileStore.selfIsGroom(context)) }
    var kundali by remember { mutableStateOf<KundaliRecord?>(null) }
    var couple by remember { mutableStateOf<CoupleRecord?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var showKootas by remember { mutableStateOf(false) }
    val qs: List<QuestionRecord> = remember { questions() }
    var question by remember { mutableStateOf("will_it_return") }
    var asked by remember { mutableStateOf<String?>(null) }
    var asking by remember { mutableStateOf(false) }
    var coupleAnswer by remember { mutableStateOf<CoupleAnswerRecord?>(null) }
    var showWhy by remember { mutableStateOf(false) }

    val partner: BirthProfile? = partners.getOrNull(selected)

    LaunchedEffect(partner, selfIsGroom, settings) {
        kundali = null
        couple = null
        coupleAnswer = null
        val p = partner ?: return@LaunchedEffect
        try {
            val engine = EngineProvider.get(context)
            kundali = withContext(Dispatchers.Default) {
                engine.kundali(
                    profile.birthInstant(), profile.latitude, profile.longitude,
                    p.birthInstant(), p.latitude, p.longitude,
                    settings.houseSystem, settings.ayanamsa, selfIsGroom,
                )
            }
            couple = withContext(Dispatchers.Default) {
                engine.couple(
                    profile.birthInstant(), profile.latitude, profile.longitude,
                    p.birthInstant(), p.latitude, p.longitude,
                    place.latitude, place.longitude,
                    settings.houseSystem, settings.ayanamsa, selfIsGroom, Instants.now(),
                )
            }
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }
    LaunchedEffect(asked) {
        val p = partner ?: return@LaunchedEffect
        if (asked == null) return@LaunchedEffect
        asking = true
        try {
            val engine = EngineProvider.get(context)
            coupleAnswer = withContext(Dispatchers.Default) {
                engine.coupleAsk(
                    profile.birthInstant(), profile.latitude, profile.longitude,
                    p.birthInstant(), p.latitude, p.longitude,
                    place.latitude, place.longitude,
                    settings.houseSystem, settings.ayanamsa, question, Instants.now(),
                )
            }
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
        asking = false
        showWhy = false
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("US", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(tr("The two of you"), style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "Put the other person's birth beside yours. How well you two match, how things stand between you today, the months behind and ahead, what will help, and any question about the two of you.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        ErrorLine(error)

        SectionLabel(tr("With"))
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((i, p) in partners.withIndex()) {
                Chip(p.name.uppercase(), selected = i == selected, onClick = { selected = i; ProfileStore.selectPartner(context, i) })
            }
            Chip("+ ADD", selected = false, onClick = onAddPartner)
            Chip("PASTE A SHARED PERSON", selected = false, onClick = {
                val cm = context.getSystemService(android.content.Context.CLIPBOARD_SERVICE) as android.content.ClipboardManager
                val text = cm.primaryClip?.getItemAt(0)?.coerceToText(context)?.toString()
                val p = com.kshana.app.data.PersonLink.parse(text)
                pasteNote = if (p == null) "Copy the message with the kshana://person link first, then tap this again." else {
                    if (com.kshana.app.data.PersonLink.add(context, p)) { partners = ProfileStore.partners(context); "Added ${p.name}." } else "${p.name} is already here."
                }
            })
        }
        pasteNote?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky) }
        if (partners.isNotEmpty()) Text("A DATE FOR ALL OF US ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onFamilyDates).padding(vertical = 4.dp))
        if (partner == null) {
            Spacer(Modifier.height(8.dp))
            Text("No one here yet.", fontFamily = FontFamily.Serif, fontSize = 20.sp, color = KshanaColors.Text)
            Text("Add a partner, a spouse, a child, a friend: a date of birth, a time if known, a place. It stays on this phone.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            GoldButton("ADD THE OTHER PERSON", onClick = onAddPartner, modifier = Modifier.fillMaxWidth())
            return@Column
        }
        // Family mode: everyone added, with their relation and how their day reads.
        var relVersion by remember { mutableStateOf(0) }
        val relation = remember(partner.name, relVersion) { ProfileStore.relationOf(context, partner.name) }
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            Text("They are my", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            for (r in ProfileStore.relations) Chip(r.uppercase(), selected = relation == r, onClick = { ProfileStore.saveRelation(context, partner.name, r); relVersion++ })
        }
        if (partners.size >= 2) {
            PeopleGrid(partners, place, settings, onPick = { i -> selected = i; ProfileStore.selectPartner(context, i) })
            MatchTwo(partners, settings, onRemedy)
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            Text("Count me as", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            Chip("GROOM", selected = selfIsGroom, onClick = { selfIsGroom = true; ProfileStore.saveSelfIsGroom(context, true) })
            Chip("BRIDE", selected = !selfIsGroom, onClick = { selfIsGroom = false; ProfileStore.saveSelfIsGroom(context, false) })
        }
        Text(
            "${partner.name} · %d %s %d · %02d:%02d · %s".format(partner.day, MONTH_SHORT[(partner.month - 1).coerceIn(0, 11)], partner.year, partner.hour, partner.minute, partner.placeName),
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )

        // Already together, or thinking of it: the match is read differently.
        var togetherVersion by remember { mutableStateOf(0) }
        val together = remember(partner.name, togetherVersion, relVersion) { ProfileStore.isTogether(context, partner.name) }
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            Text("We are", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            Chip("ALREADY TOGETHER", selected = together, onClick = { ProfileStore.saveTogether(context, partner.name, true); togetherVersion++ })
            Chip("THINKING OF IT", selected = !together, onClick = { ProfileStore.saveTogether(context, partner.name, false); togetherVersion++ })
        }

        val k = kundali
        if (k == null) {
            if (error == null) Working("putting the two of you side by side…")
        } else {
            val me = profile.name.substringBefore(' ').ifBlank { "You" }
            val them = partner.name.substringBefore(' ')
            Column(
                Modifier.fillMaxWidth().glass(18.dp).padding(18.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                    ScoreRing(k.total / 36.0 * 100.0, size = 84.dp, stroke = 7.dp, label = "of 36")
                    Column(Modifier.weight(1f)) {
                        Text(if (together) "YOUR BOND · ${"%.1f".format(k.total)} points" else "THE MATCH · ${"%.1f".format(k.total)} points", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                        Text(
                            if (together) k.togetherVerdict else k.verdict,
                            style = MaterialTheme.typography.displayMedium,
                            color = if (together) KshanaColors.Gold else scoreColor(k.total / 36.0 * 100.0),
                        )
                    }
                }
                Text(personalise(if (together) k.togetherHeadline else k.headline, me, them), fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Text)
                if (!together) {
                    Text(personalise(k.kujaWords, me, them), style = MaterialTheme.typography.bodyMedium, color = if (k.kujaA != k.kujaB) KshanaColors.Ember else KshanaColors.TextSoft)
                    if (k.care.isNotEmpty()) Text(
                        "A low score is not a no. It marks where two people need to take care of each other; every part of it has steps below that ease it.",
                        style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
                    )
                } else {
                    Text(
                        "Many happy couples carry low scores. The score shows where to take care, not how your marriage will go: that is decided by how you treat each other.",
                        style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
                    )
                }
            }
            if (k.strengths.isNotEmpty()) {
                SectionLabel("Where your bond is strong")
                for (s in k.strengths) Bullet(personalise(s, me, them), KshanaColors.Leaf)
            }
            if (k.care.isNotEmpty()) {
                SectionLabel(if (together) "Where your bond asks for care" else "Where you would need to take care")
                for (c in k.care) BondCareCard(c, me, them)
                QuietButton("MORE REMEDIES FOR LOVE", onClick = { onRemedy("Love") }, modifier = Modifier.fillMaxWidth())
            }
            Text(
                if (showKootas) "LESS −" else "THE EIGHT POINTS OF THE MATCH +",
                style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                modifier = Modifier.clickable { showKootas = !showKootas }.padding(vertical = 4.dp),
            )
            if (showKootas) {
                for (x in k.kootas) {
                    HorizontalDivider(color = KshanaColors.LineSoft)
                    Row(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalAlignment = Alignment.Top) {
                        Column(Modifier.width(96.dp)) {
                            Text(x.name, fontFamily = FontFamily.Serif, fontSize = 16.sp, color = KshanaColors.Text)
                            Text("${trimPoints(x.points)} / ${trimPoints(x.max)}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = scoreColor(x.points / x.max * 100.0))
                        }
                        Column(Modifier.weight(1f)) {
                            Box(Modifier.fillMaxWidth().height(5.dp).clip(RoundedCornerShape(2.dp)).background(KshanaColors.Line)) {
                                Box(Modifier.fillMaxWidth((x.points / x.max).toFloat().coerceIn(0.02f, 1f)).height(5.dp).background(scoreColor(x.points / x.max * 100.0)))
                            }
                            Text(x.words.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.padding(top = 4.dp))
                        }
                    }
                }
            }
        }

        val c = couple
        if (c == null) {
            if (k != null && error == null) Working("reading both of you, and three years of months…")
        } else {
            SectionLabel("Right now, together")
            Column(
                Modifier.fillMaxWidth().glass(18.dp).padding(18.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                    ScoreRing(c.together, size = 84.dp, stroke = 7.dp, label = bandWord(c.band))
                    Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        PersonScore(profile.name.substringBefore(' ').ifBlank { "You" }, c.aLove, KshanaColors.Sky)
                        PersonScore(partner.name.substringBefore(' '), c.bLove, KshanaColors.Leaf)
                        Text("how love feels for each of you today", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                    }
                }
                val me = profile.name.substringBefore(' ').ifBlank { "You" }
                val them = partner.name.substringBefore(' ')
                Text(personalise(c.headline.substringBefore(" The heavier side"), me, them), fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Text)
                if (c.aSaturnActive || c.bSaturnActive) {
                    Text(personalise(c.notes.firstOrNull() ?: "", me, them), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Ember)
                }
            }

            SectionLabel("Twelve months back, two years ahead")
            CoupleChart(c)
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text(c.series.firstOrNull()?.let { "${dayMon(it.at)} ${localText(it.at, YEAR_ONLY)}" } ?: "", fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted)
                Text("now", fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Gold)
                Text(c.series.lastOrNull()?.let { "${dayMon(it.at)} ${localText(it.at, YEAR_ONLY)}" } ?: "", fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Muted)
            }
            Row(horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                Legend(profile.name.substringBefore(' ').ifBlank { "You" }, KshanaColors.Sky)
                Legend(partner.name.substringBefore(' '), KshanaColors.Leaf)
                Legend("together", KshanaColors.Gold)
            }
            c.easesAt?.let { Bullet("The pair climbs to ${bandWord(scoreBand(c.easesScore)).lowercase()} from ${localText(it, DATE_ONLY)}.", KshanaColors.Leaf) }
            c.peakAt?.let { Bullet("Best month ahead: ${localText(it, DATE_ONLY)} at ${c.peakScore.toInt()}.", KshanaColors.Gold) }
            c.lowAt?.let { Bullet("Lowest month ahead: ${localText(it, DATE_ONLY)} at ${c.lowScore.toInt()}. Hold what you have there.", KshanaColors.Ember) }

            SectionLabel("What each of you is going through")
            val me = profile.name.substringBefore(' ').ifBlank { "You" }
            val them = partner.name.substringBefore(' ')
            for (p in c.togetherWords) Text(personalise(p, me, them), fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 25.sp, color = KshanaColors.Text)
            PersonBriefing(me, c.aBriefing, KshanaColors.Sky)
            PersonBriefing(them, c.bBriefing, KshanaColors.Leaf)

            SectionLabel("What to do about it")
            if (k?.care.isNullOrEmpty() && c.aBriefing.remedy == null && c.bBriefing.remedy == null) {
                Text("Nothing about the two of you needs a remedy today. Keep doing what you do for each other.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            }
            if (!k?.care.isNullOrEmpty()) {
                Text("For the match itself, the steps are above under each part that asks for care.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            }
            for ((who, rb) in listOf(me to c.aBriefing.remedy, them to c.bBriefing.remedy)) {
                if (rb == null) continue
                Text("For $who", style = MaterialTheme.typography.titleMedium, color = KshanaColors.Gold)
                Text(rb.cause, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                for (t in rb.top) {
                    Row(Modifier.padding(vertical = 2.dp), verticalAlignment = Alignment.Top) {
                        Text(kindWord(t.kind), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.width(78.dp).padding(top = 3.dp))
                        Column(Modifier.weight(1f)) {
                            Text(t.title, style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text)
                            Text(t.why, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                        }
                    }
                }
                HowThisHelps(remember(rb.planet) { uniffi.kshana_ffi.remedyHowItHelps(rb.planet, "dasha", null) })
            }
            QuietButton("THE FULL REMEDY FOR LOVE", onClick = { onRemedy("Love") }, modifier = Modifier.fillMaxWidth())

            SectionLabel("What weighs, what lifts")
            Text(profile.name.substringBefore(' ').ifBlank { "You" }, fontFamily = FontFamily.Serif, fontSize = 17.sp, color = KshanaColors.Sky)
            if (c.aCause.isNotBlank()) Text(c.aCause.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
            for (r in c.aRelief) ReliefRow(r)
            Spacer(Modifier.height(4.dp))
            Text(partner.name.substringBefore(' '), fontFamily = FontFamily.Serif, fontSize = 17.sp, color = KshanaColors.Leaf)
            if (c.bCause.isNotBlank()) Text(c.bCause.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
            for (r in c.bRelief) ReliefRow(r)
            for (n in c.notes.drop(1)) Text(personalise(n, profile.name.substringBefore(' ').ifBlank { "You" }, partner.name.substringBefore(' ')), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)

            SectionLabel("Ask about the two of you")
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                for (row in qs.chunked(2)) {
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        for (q in row) {
                            OptionBox(coupleWords(q.key, q.words), selected = question == q.key, onClick = { question = q.key; coupleAnswer = null }, modifier = Modifier.weight(1f))
                        }
                        if (row.size == 1) Spacer(Modifier.weight(1f))
                    }
                }
            }
            GoldButton(if (asking) "WORKING IT OUT…" else "ASK · ABOUT US", enabled = !asking, onClick = { asked = "$question|${System.currentTimeMillis()}" }, modifier = Modifier.fillMaxWidth())
            val ca = coupleAnswer
            if (ca != null) {
                Column(
                    Modifier.fillMaxWidth().glass(18.dp).padding(18.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                        ScoreRing(ca.score, size = 84.dp, stroke = 7.dp, label = "${ca.confidence.toInt()}% sure")
                        Column(Modifier.weight(1f)) {
                            Text(coupleWords(question, qs.firstOrNull { it.key == question }?.words ?: question), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                            Text(ca.verdict, style = MaterialTheme.typography.titleLarge, color = scoreColor(ca.score))
                            Text(uniffi.kshana_ffi.sureWords(com.kshana.app.data.Lang.code, ca.confidence), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                        }
                    }
                    Text(personalise(ca.headline, profile.name.substringBefore(' ').ifBlank { "You" }, partner.name.substringBefore(' ')), fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Text)
                    val ws = ca.whenStart
                    val we = ca.whenEnd
                    if (ws != null && we != null) {
                        Text("When it opens for both of you", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                        Text("${whenText(ws)} – ${whenText(we)}", fontFamily = FontFamily.Monospace, fontSize = 15.sp, color = KshanaColors.Sky)
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                        MiniStat("you now", ca.a.now)
                        MiniStat("you ahead", ca.a.aheadBest)
                        MiniStat("them now", ca.b.now)
                        MiniStat("them ahead", ca.b.aheadBest)
                    }
                }
                Text(
                    if (showWhy) "LESS −" else "WHAT LIES BEHIND IT +",
                    style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                    modifier = Modifier.clickable { showWhy = !showWhy }.padding(vertical = 4.dp),
                )
                if (showWhy) {
                    Text("You", fontFamily = FontFamily.Serif, fontSize = 16.sp, color = KshanaColors.Sky)
                    for (r in ca.a.reasons) Bullet(r, KshanaColors.Sky)
                    Text("Them", fontFamily = FontFamily.Serif, fontSize = 16.sp, color = KshanaColors.Leaf)
                    for (r in ca.b.reasons) Bullet(r, KshanaColors.Leaf)
                }
                if (ca.score < 56) QuietButton("REMEDY FOR LOVE", onClick = { onRemedy("Love") }, modifier = Modifier.fillMaxWidth())
            }
        }

        SectionLabel("Share ${partner.name.substringBefore(' ').lowercase().replaceFirstChar { it.uppercase() }}")
        Text("Send these birth details to someone else who uses Kshana, by message or by showing the code. Only these details go, and only to whom you send them.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        var showQr by remember(partner) { mutableStateOf(false) }
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            QuietButton("SEND AS A MESSAGE", onClick = { runCatching { context.startActivity(com.kshana.app.data.PersonLink.shareIntent(partner, profile.name)) } }, modifier = Modifier.weight(1f))
            QuietButton(if (showQr) "HIDE CODE" else "SHOW CODE", onClick = { showQr = !showQr }, modifier = Modifier.weight(1f))
        }
        if (showQr) {
            val bmp = remember(partner) { com.kshana.app.data.PersonLink.qr(com.kshana.app.data.PersonLink.of(partner)) }
            if (bmp != null) androidx.compose.foundation.Image(bmp.asImageBitmap(), contentDescription = "code", modifier = Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)))
        }
        Spacer(Modifier.height(8.dp))
        QuietButton("REMOVE ${partner.name.uppercase()}", onClick = {
            ProfileStore.removePartner(context, selected)
            partners = ProfileStore.partners(context)
            selected = ProfileStore.selectedPartner(context).coerceIn(0, (partners.size - 1).coerceAtLeast(0))
        })
        Text("Everything about the two of you is worked out on this phone. Nothing leaves it.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    }
}

private val MONTH_SHORT = listOf("Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec")

/** Everyone added, each with how their day reads this hour. */
@Composable
private fun PeopleGrid(people: List<BirthProfile>, place: Place, settings: EngineSettings, onPick: (Int) -> Unit) {
    val context = LocalContext.current
    val hourKey = System.currentTimeMillis() / 3_600_000L
    var moments by remember { mutableStateOf<Map<String, uniffi.kshana_ffi.MomentRecord>>(emptyMap()) }
    LaunchedEffect(hourKey, people) {
        runCatching {
            val engine = EngineProvider.get(context)
            val out = HashMap<String, uniffi.kshana_ffi.MomentRecord>()
            for (p in people) {
                val k = "${p.name}|$hourKey|${place.name}"
                out[p.name] = PeopleCache.moments[k] ?: withContext(Dispatchers.Default) {
                    engine.forecastMoment(p.birthInstant(), p.latitude, p.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, Instants.now(), com.kshana.app.data.Lang.code)
                }.also { PeopleCache.moments[k] = it }
            }
            moments = out
        }
    }
    SectionLabel("Your people · this hour")
    for ((i, p) in people.withIndex()) {
        val pm = moments[p.name]
        Row(
            Modifier.fillMaxWidth().glass(12.dp).clickable { onPick(i) }.padding(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text("${p.name.substringBefore(' ')} · ${ProfileStore.relationOf(context, p.name)}", style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text)
                if (pm != null) {
                    val top = pm.lives.maxByOrNull { it.score }
                    val low = pm.lives.minByOrNull { it.score }
                    Text("Going well for them: ${top?.life?.let { lifeName(it).lowercase() }}. Go gently today with: ${low?.life?.let { lifeName(it).lowercase() }}.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.TextSoft)
                } else {
                    Text("reading…", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                }
            }
            if (pm != null) ScoreRing(pm.overall, size = 44.dp, stroke = 4.dp)
        }
    }
}

/** Pick two of your people and read their match. */
@Composable
private fun MatchTwo(people: List<BirthProfile>, settings: EngineSettings, onRemedy: (String) -> Unit) {
    val context = LocalContext.current
    var a by remember { mutableStateOf(0) }
    var b by remember { mutableStateOf(1) }
    var k by remember { mutableStateOf<KundaliRecord?>(null) }
    var open by remember { mutableStateOf(false) }
    Text(if (open) "MATCH TWO OF THEM −" else "MATCH TWO OF THEM +", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable { open = !open }.padding(vertical = 4.dp))
    if (!open) return
    Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for ((i, p) in people.withIndex()) Chip(p.name.substringBefore(' ').uppercase(), selected = i == a, onClick = { a = i; k = null })
    }
    Text("with", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for ((i, p) in people.withIndex()) Chip(p.name.substringBefore(' ').uppercase(), selected = i == b, onClick = { b = i; k = null })
    }
    LaunchedEffect(a, b, open) {
        if (a == b) { k = null; return@LaunchedEffect }
        runCatching {
            val engine = EngineProvider.get(context)
            val pa = people[a]
            val pb = people[b]
            k = withContext(Dispatchers.Default) {
                engine.kundali(pa.birthInstant(), pa.latitude, pa.longitude, pb.birthInstant(), pb.latitude, pb.longitude, settings.houseSystem, settings.ayanamsa, true)
            }
        }
    }
    val r = k
    if (a == b) {
        Text("Pick two different people.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    } else if (r != null) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
            ScoreRing(r.total / 36.0 * 100.0, size = 64.dp, stroke = 6.dp, label = "of 36")
            Column(Modifier.weight(1f)) {
                val na = people[a].name.substringBefore(' ')
                val nb = people[b].name.substringBefore(' ')
                val tog = ProfileStore.isTogether(context, people[a].name) && ProfileStore.isTogether(context, people[b].name)
                Text("$na and $nb · ${if (tog) r.togetherVerdict else r.verdict}", style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text)
                Text(byNames(if (tog) r.togetherHeadline else r.headline, na, nb), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
            }
        }
        for (c in r.care) BondCareCard(c, people[a].name.substringBefore(' '), people[b].name.substringBefore(' '), firstIsYou = false)
        if (r.care.isNotEmpty()) QuietButton("MORE REMEDIES FOR LOVE", onClick = { onRemedy("Love") }, modifier = Modifier.fillMaxWidth())
        Text("The first is counted as the groom.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    }
}

/**
 * One part of the match that asks for care: what it brings to two people living
 * together, what each can do for the other, what helps (deeds first, prayer if they
 * wish), and the chart reason one tap away.
 */
@Composable
private fun BondCareCard(c: uniffi.kshana_ffi.BondCareRecord, me: String, them: String, firstIsYou: Boolean = true) {
    val say: (String) -> String = { t -> if (firstIsYou) personalise(t, me, them) else byNames(t, me, them) }
    var showHow by remember(c.key) { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth().glass(16.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(c.area.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(say(c.living), fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 25.sp, color = KshanaColors.Text)

        Text("WHAT EACH OF YOU CAN DO FOR THE OTHER", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Sky, modifier = Modifier.padding(top = 4.dp))
        for (s in c.care) Bullet(say(s), KshanaColors.Sky)

        val deeds = c.remedies.filter { !it.optional }
        val prayer = c.remedies.filter { it.optional }
        if (deeds.isNotEmpty()) {
            Text("WHAT HELPS · DEEDS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Leaf, modifier = Modifier.padding(top = 4.dp))
            for (r in deeds) RemedyLine(r.title, r.why, KshanaColors.Leaf)
        }
        if (prayer.isNotEmpty()) {
            Text("PRAYER, IF YOU WISH", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.padding(top = 4.dp))
            for (r in prayer) RemedyLine(r.title, r.why, KshanaColors.Gold)
        }

        Text(
            if (showHow) "HOW THIS HELPS −" else "HOW THIS HELPS +",
            style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
            modifier = Modifier.clickable { showHow = !showHow }.padding(top = 6.dp, bottom = 2.dp),
        )
        androidx.compose.animation.AnimatedVisibility(showHow) {
            Text(say(c.howItHelps), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        }
    }
}

@Composable
private fun RemedyLine(title: String, why: String, c: Color) {
    Row(Modifier.fillMaxWidth().padding(vertical = 2.dp), verticalAlignment = Alignment.Top) {
        Box(Modifier.padding(top = 8.dp).size(6.dp).clip(CircleShape).background(c))
        Spacer(Modifier.width(10.dp))
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text)
            Text(why, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        }
    }
}

/** Like [personalise] when neither of the two is the person using the app. */
private fun byNames(s: String, aRaw: String, bRaw: String): String {
    val a = properName(aRaw)
    val b = properName(bRaw)
    return s.replace("The first person's", "$a's").replace("the first person's", "$a's")
        .replace("The second person's", "$b's").replace("the second person's", "$b's")
        .replace("The first person", a).replace("the first person", a)
        .replace("The second person", b).replace("the second person", b)
        .replace("First person:", "$a:").replace("Second person:", "$b:")
}

/** The engine speaks of "the first chart" and "the second"; the screen uses names. */
private fun personalise(s: String, meRaw: String, themRaw: String): String {
    val me = properName(meRaw)
    val them = properName(themRaw)
    return s.replace("The first person's", "$me's")
        .replace("The second person's", "$them's")
        .replace("The first person", me)
        .replace("The second person", them)
        .replace("the second's", "$them's")
        .replace("the second is", "$them is")
        .replace("for the first person and", "for you and")
        .replace("for the second.", "for $them.")
        .replace("the first person's", "your")
        .replace("the second person's", "$them's")
        .replace("First person:", "$me:")
        .replace("Second person:", "$them:")
        .replace("the first person", "you")
        .replace("the second person", them)
}

/** "VYSAKH" → "Vysakh". */
private fun properName(s: String): String = s.lowercase().replaceFirstChar { it.uppercase() }

private fun trimPoints(x: Double): String = if (x == x.toLong().toDouble()) x.toLong().toString() else "%.1f".format(x)

private fun ordinalText(n: Int): String = when (n) {
    1 -> "1st"; 2 -> "2nd"; 3 -> "3rd"; else -> "${n}th"
}

private fun scoreBand(score: Double): String = when {
    score >= 70 -> "bright"; score >= 56 -> "good"; score >= 44 -> "steady"; score >= 32 -> "slow"; else -> "hold"
}

private fun coupleWords(key: String, fallback: String): String = when (key) {
    "will_it_happen" -> "Will we be together?"
    "when" -> "When do we come close again?"
    "good_time_now" -> "Is now a good time to talk?"
    "should_i" -> "Should I take the step?"
    "will_it_return" -> "Will they come back?"
    "will_it_recover" -> "Will it be set right between us?"
    else -> fallback
}

/** One person's reading under Us, in plain words. */
@Composable
private fun PersonBriefing(name: String, b: uniffi.kshana_ffi.BriefingRecord, c: Color) {
    var open by remember { mutableStateOf(false) }
    Column(
        Modifier.fillMaxWidth().glass(14.dp).padding(14.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(8.dp).clip(CircleShape).background(c))
            Spacer(Modifier.width(8.dp))
            Text(name, fontFamily = FontFamily.Serif, fontSize = 18.sp, color = c)
        }
        val shown = if (open) b.paragraphs else b.paragraphs.take(1)
        for (p in shown) Text(p.replace("Their birth Moon", "$name's birth Moon").replace("They are in", "$name is in"), fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 24.sp, color = KshanaColors.Text)
        Text(
            if (open) "LESS −" else "THE WHOLE READING FOR ${name.uppercase()} +",
            style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
            modifier = Modifier.clickable { open = !open }.padding(vertical = 2.dp),
        )
    }
}

@Composable
private fun PersonScore(name: String, v: Double, c: Color) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(8.dp).clip(CircleShape).background(c))
        Spacer(Modifier.width(8.dp))
        Text(name, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text, modifier = Modifier.width(90.dp))
        Text(v.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 18.sp, color = scoreColor(v))
    }
}

@Composable
private fun Legend(name: String, c: Color) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(8.dp).clip(CircleShape).background(c))
        Spacer(Modifier.width(6.dp))
        Text(name, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
    }
}

@Composable
private fun MiniStat(label: String, v: Double) {
    Column {
        Text(label.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        Text(v.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 16.sp, color = scoreColor(v))
    }
}

@Composable
internal fun Bullet(text: String, c: Color) {
    Row(Modifier.padding(vertical = 3.dp), verticalAlignment = Alignment.Top) {
        Box(Modifier.padding(top = 7.dp).size(6.dp).clip(CircleShape).background(c))
        Spacer(Modifier.width(10.dp))
        Text(text.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
    }
}

@Composable
internal fun ReliefRow(r: ReliefRecord) {
    val up = r.gain >= 0
    Row(Modifier.fillMaxWidth().padding(vertical = 3.dp), verticalAlignment = Alignment.Top) {
        Text(localText(r.at, DATE_ONLY), fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky, modifier = Modifier.width(92.dp).padding(top = 2.dp))
        Text(r.what, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
        Text(signedPoints(r.gain), fontFamily = FontFamily.Monospace, fontSize = 13.sp, color = if (up) KshanaColors.Leaf else KshanaColors.Ember, modifier = Modifier.padding(start = 8.dp))
    }
}

/** Three lines: each person's love and the pair, with the present marked. */
@Composable
private fun CoupleChart(c: CoupleRecord) {
    val s = c.series
    if (s.size < 2) return
    val all = s.flatMap { listOf(it.a, it.b, it.together) }
    val lo = (all.min() - 8.0).coerceAtLeast(0.0)
    val hi = (all.max() + 8.0).coerceAtMost(100.0)
    val span = (hi - lo).coerceAtLeast(30.0)
    val bottom = if (hi - lo < 30.0) ((lo + hi) / 2 - 15.0).coerceIn(0.0, 70.0) else lo
    val nowIdx = s.indices.minByOrNull { kotlin.math.abs(s[it].at.seconds() - c.at.seconds()) } ?: 0
    Canvas(Modifier.fillMaxWidth().height(130.dp)) {
        val n = s.size
        val stepX = size.width / (n - 1)
        fun y(v: Double) = (size.height * (1 - ((v - bottom) / span).coerceIn(0.0, 1.0))).toFloat()
        for (g in listOf(32.0, 44.0, 56.0, 70.0)) {
            if (g > bottom && g < bottom + span) drawLine(KshanaColors.LineSoft, Offset(0f, y(g)), Offset(size.width, y(g)), 1f)
        }
        drawLine(KshanaColors.Gold.copy(alpha = 0.6f), Offset(nowIdx * stepX, 0f), Offset(nowIdx * stepX, size.height), 1.5f, pathEffect = PathEffect.dashPathEffect(floatArrayOf(6f, 6f)))
        fun line(pick: (uniffi.kshana_ffi.CoupleSampleRecord) -> Double, color: Color, width: Float) {
            val p = Path()
            s.forEachIndexed { i, x ->
                val o = Offset(i * stepX, y(pick(x)))
                if (i == 0) p.moveTo(o.x, o.y) else p.lineTo(o.x, o.y)
            }
            drawPath(p, color, style = Stroke(width, cap = StrokeCap.Round))
        }
        line({ it.a }, KshanaColors.Sky, 2f)
        line({ it.b }, KshanaColors.Leaf, 2f)
        line({ it.together }, KshanaColors.Gold, 3f)
        drawCircle(KshanaColors.Gold, 5f, Offset(nowIdx * stepX, y(s[nowIdx].together)))
    }
}
