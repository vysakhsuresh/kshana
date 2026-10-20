package com.kshana.app.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
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
import com.kshana.app.data.Focus
import com.kshana.app.data.Journal
import com.kshana.app.data.Lang
import com.kshana.app.data.Place
import com.kshana.app.data.Situations
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.AnswerRecord
import uniffi.kshana_ffi.BriefingRecord
import uniffi.kshana_ffi.FutureEventRecord
import uniffi.kshana_ffi.PastCheckRecord
import uniffi.kshana_ffi.RemedyTreeRecord
import uniffi.kshana_ffi.topics

/** Everything the reading loads, kept for the day so the page opens instantly afterwards. */
internal object ConsultCache {
    var key: String? = null
    var past: List<PastCheckRecord>? = null
    var future: List<FutureEventRecord>? = null
    var tree: RemedyTreeRecord? = null
    val answers = HashMap<String, AnswerRecord>()
}

/** What the Ask screen should open on, when another screen sends the person there. */
internal object AskPrefill {
    var life: String? = null
    var topic: String? = null
    var question: String? = null
}

/** Where the reading's doors lead. */
internal class ReadingDoors(
    val ask: () -> Unit,
    val remedy: (String?) -> Unit,
    val future: () -> Unit,
    val ahead: () -> Unit,
    val today: () -> Unit,
    val book: () -> Unit,
    val record: () -> Unit,
    val journal: () -> Unit,
    val how: () -> Unit,
    val life: (String) -> Unit,
)

/**
 * The reading: a sitting with an astrologer, on one page, in the order one would speak.
 * What is on your mind, answered with dates; your past, read back to you; where you
 * stand; what is coming; what will help; and how sure all of it is. Every section is
 * short, and every section has a door to the whole of it.
 */
@Composable
internal fun ConsultScreen(profile: BirthProfile, place: Place, settings: EngineSettings, doors: ReadingDoors) {
    val context = LocalContext.current
    val key = "${profile.birthInstant().taiSeconds}|${place.name}|${Lang.code}|${Focus.lives}|${java.time.LocalDate.now()}"
    if (ConsultCache.key != key) {
        ConsultCache.key = key
        ConsultCache.past = null
        ConsultCache.future = null
        ConsultCache.tree = null
        ConsultCache.answers.clear()
    }
    var brief by remember { mutableStateOf(BriefingCache.record) }
    var past by remember { mutableStateOf(ConsultCache.past) }
    var future by remember { mutableStateOf(ConsultCache.future) }
    var tree by remember { mutableStateOf(ConsultCache.tree) }
    var picking by remember { mutableStateOf(false) }
    var journalVersion by remember { mutableStateOf(0) }

    LaunchedEffect(key) {
        val engine = EngineProvider.get(context)
        val now = Instants.now()
        runCatching {
            if (brief == null) {
                brief = withContext(Dispatchers.Default) {
                    engine.briefing(profile.birthInstant(), profile.latitude, profile.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, Focus.lives, now, Lang.code)
                }
            }
        }
        runCatching {
            if (past == null) past = withContext(Dispatchers.Default) {
                engine.pastChecks(profile.birthInstant(), profile.latitude, profile.longitude, settings.houseSystem, settings.ayanamsa, now, Lang.code, Focus.lives)
            }.also { ConsultCache.past = it }
        }
        runCatching {
            if (future == null) future = withContext(Dispatchers.Default) {
                engine.future(profile.birthInstant(), profile.latitude, profile.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, now, 12u, Lang.code, Focus.lives)
            }.also { ConsultCache.future = it }
        }
        runCatching {
            if (tree == null) tree = withContext(Dispatchers.Default) {
                engine.remedyTree(profile.birthInstant(), profile.latitude, profile.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, null, now, Lang.code, Focus.lives)
            }.also { ConsultCache.tree = it }
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        val first = profile.name.substringBefore(' ').lowercase().replaceFirstChar { it.uppercase() }.ifBlank { "Your" }
        Text("YOUR READING", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(if (first == "Your") "Your reading" else "$first's reading", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "As an astrologer would sit with you: what is on your mind first, then your past, where you stand, what is coming and what will help. Tap any door to go further.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        Column(
            Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).border(1.dp, KshanaColors.Sky.copy(alpha = 0.35f), RoundedCornerShape(12.dp)).padding(12.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            Text("HOW THIS READING IS MADE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Sky)
            Text(
                "Kshana works offline. Every answer here comes from your birth time and the positions of the sky, worked out on this phone; nothing is sent anywhere and no person or chatbot writes it. It answers a set list of questions about life. When a question is outside that list, or it is not sure what you meant, it tells you so plainly and offers the closest questions it can answer, rather than guess.",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.TextSoft,
            )
        }
        Row(
            Modifier.fillMaxWidth().clip(RoundedCornerShape(26.dp)).border(1.dp, KshanaColors.Gold.copy(alpha = 0.6f), RoundedCornerShape(26.dp)).clickable(onClick = doors.ask).padding(horizontal = 18.dp, vertical = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text("Ask anything, in your own words…", style = MaterialTheme.typography.bodyLarge, color = KshanaColors.Muted, modifier = Modifier.weight(1f))
            Text("🎙", fontSize = 18.sp)
        }

        // 1. What is on your mind.
        SectionLabel("What is on your mind")
        val mine = Situations.mine
        if (mine.isEmpty() && !picking) {
            Column(Modifier.fillMaxWidth().glass(16.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Tell the app what you are going through, once.", fontFamily = FontFamily.Serif, fontSize = 18.sp, color = KshanaColors.Text)
                Text("A marriage that is delayed, a job you are waiting for, a debt, an illness, a partner who left. Every reading after this speaks to it, with dates.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                GoldButton("CHOOSE WHAT IS ON MY MIND", onClick = { picking = true }, modifier = Modifier.fillMaxWidth())
            }
        }
        for (s in mine) {
            if (s.key == Situations.CHART_FEAR) ChartFearCard(brief, tree, doors) else SituationAnswer(profile, place, settings, s.key, s.since, doors)
        }
        if (picking) SituationPicker(onDone = { picking = false })
        else if (mine.isNotEmpty()) Door("Add or change what is on my mind", onClick = { picking = true })

        // 2. Your past, read back.
        SectionLabel("Your past, read back")
        val p = past
        if (p == null) Working("reading your last six years…")
        else {
            Text("From your birth time alone, these stretches stand out. Say whether they happened; it keeps the reading honest.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            val events = remember(journalVersion) { Journal.all(context) }
            for (c in p) PastLine(c, events, onAnswer = { journalVersion++ })
        }
        Door("Your whole life story, year by year", onClick = doors.book)

        // 3. Where you stand now.
        SectionLabel("Where you stand now")
        val b = brief
        if (b == null) Working("reading where you stand…")
        else {
            Text(b.paragraphs.firstOrNull() ?: "", fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 25.sp, color = KshanaColors.Text)
            Text(b.moment.taraWords.replaceFirstChar { it.uppercase() }.trimEnd('.') + ".", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        }
        Door("Today, hour by hour", onClick = doors.today)
        Door("This week, month and year", onClick = doors.ahead)

        // 4. What is coming.
        SectionLabel("What is coming this year")
        val f = future
        if (f == null) Working("looking ahead, month by month…")
        else {
            val top = f.sortedByDescending { it.strength }.take(5).sortedBy { it.from.taiSeconds }
            if (top.isEmpty()) Text("A quiet year: nothing strong opens or presses for the parts of life you chose.", style = MaterialTheme.typography.bodyLarge, color = KshanaColors.TextSoft)
            for (e in top) {
                // A pressing stretch opens what helps with it.
                val presses = e.kind == "pressure" || e.kind == "test_begins" || e.kind == "test_returns"
                Row(
                    Modifier.fillMaxWidth().then(if (presses) Modifier.clickable { doors.remedy(e.life) } else Modifier).padding(vertical = 4.dp),
                    verticalAlignment = Alignment.Top,
                ) {
                    Box(Modifier.padding(top = 7.dp).size(8.dp).clip(CircleShape).background(if (e.kind == "opening") KshanaColors.Leaf else if (e.kind == "chapter") KshanaColors.Gold else KshanaColors.Ember))
                    Spacer(Modifier.size(10.dp))
                    Column(Modifier.weight(1f)) {
                        Text(e.headline, style = MaterialTheme.typography.bodyLarge, color = KshanaColors.Text)
                        if (presses) Text("WHAT HELPS ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.padding(top = 2.dp))
                    }
                }
            }
        }
        Door("The full timeline, up to five years", onClick = doors.future)

        // 5. What will help.
        SectionLabel("What will help")
        val t = tree
        if (t == null) Working("finding what will help…")
        else {
            Text(t.headline, fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 25.sp, color = KshanaColors.Text)
            val deeds = t.causes.firstOrNull()?.branches?.firstOrNull { it.key == "deeds" }?.items.orEmpty().take(3)
            for (d in deeds) {
                Row(Modifier.padding(vertical = 3.dp), verticalAlignment = Alignment.Top) {
                    Text("•", color = KshanaColors.Gold, modifier = Modifier.padding(end = 10.dp))
                    Column {
                        Text(d.title, style = MaterialTheme.typography.bodyLarge, color = KshanaColors.Text)
                        Text(d.why, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                    }
                }
            }
            Text("Deeds first, for any faith or none. Nothing here costs money unless you choose to give.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
        Door("Everything that helps, and why", onClick = { doors.remedy(tree?.life) })

        // 6. How sure is this.
        SectionLabel("How sure is this?")
        val record = remember { com.kshana.app.data.PredictionStore.all(context).filter { it.happened != null } }
        val right = record.count { it.happened == true }
        val confirmed = remember(journalVersion) { Journal.all(context).count { it.note.startsWith("Confirmed") } }
        Text(
            buildString {
                append(if (record.isEmpty()) "No predictions graded yet." else "Right $right of ${record.size} times you checked.")
                append(" ")
                append(if (confirmed > 0) "You confirmed $confirmed stretches of your past." else "Confirm a stretch of your past above to begin your record.")
                append(" It is guidance, never a guarantee; what you do still counts most.")
            },
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
        )
        Door("Your record, right and wrong", onClick = doors.record)
        Door("Your journal", onClick = doors.journal)
        Door("How Kshana works it out", onClick = doors.how)
    }
}

/** One thing on the person's mind, answered: the verdict, the dates, how sure, and the doors. */
@Composable
private fun SituationAnswer(profile: BirthProfile, place: Place, settings: EngineSettings, key: String, since: String?, doors: ReadingDoors) {
    val context = LocalContext.current
    val kind = Situations.kind(key) ?: return
    var a by remember(key) { mutableStateOf(ConsultCache.answers[key]) }
    var why by remember(key) { mutableStateOf(false) }
    LaunchedEffect(key) {
        if (a != null) return@LaunchedEffect
        runCatching {
            val engine = EngineProvider.get(context)
            val r = withContext(Dispatchers.Default) {
                if (kind.topic != null && topics().any { it.key == kind.topic }) {
                    engine.askTopic(profile.birthInstant(), profile.latitude, profile.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, kind.topic, kind.question, Instants.now())
                } else {
                    engine.ask(profile.birthInstant(), profile.latitude, profile.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, kind.life, kind.question, Instants.now())
                }
            }
            ConsultCache.answers[key] = r
            a = r
        }
    }
    Column(Modifier.fillMaxWidth().glass(18.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text("${lifeGlyph(kind.life)} ${kind.says.uppercase()}", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        val r = a
        if (r == null) {
            Working("working it out…")
            return@Column
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(r.verdict, style = MaterialTheme.typography.displayMedium, color = scoreColor(r.score), modifier = Modifier.weight(1f))
            ScoreRing(r.score, size = 56.dp, stroke = 5.dp)
        }
        Text(uniffi.kshana_ffi.sureWords(Lang.code, r.confidence), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        Text(r.headline, fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Text)
        val ws = r.whenStart
        val we = r.whenEnd
        if (ws != null && we != null) {
            Column(Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).background(KshanaColors.Gold.copy(alpha = 0.08f)).padding(12.dp)) {
                Text(r.whenWords.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text("${whenText(ws)} – ${whenText(we)}", fontFamily = FontFamily.Monospace, fontSize = 16.sp, color = KshanaColors.Text)
            }
        }
        since?.let { Text("Going on since $it.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted) }
        if (why) {
            for (x in r.reasons) Text("• " + x.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        }
        Door(if (why) "Hide the reasons" else "Why this answer", onClick = { why = !why })
        Door("What will help with this", onClick = { doors.remedy(kind.life) })
        Door("Ask more about it", onClick = {
            AskPrefill.life = kind.life
            AskPrefill.topic = kind.topic
            AskPrefill.question = kind.question
            doors.ask()
        })
    }
}

/** The fear that something is wrong in the chart, answered with what is actually there. */
@Composable
private fun ChartFearCard(brief: BriefingRecord?, tree: RemedyTreeRecord?, doors: ReadingDoors) {
    Column(Modifier.fillMaxWidth().glass(18.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("SOMEONE SAID SOMETHING IS WRONG IN MY CHART", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        val b = brief
        if (b == null) {
            Working("looking at your chart…")
            return@Column
        }
        Text(
            if (b.saturnActive) "Here is what is actually there: ${firstSentence(b.saturnWords.replaceFirstChar { it.uppercase() })} It is a stretch of time, not a curse. It began, and it ends on a known date."
            else "Here is what is actually there: no long testing time is running for you now. Nothing in your chart is pressing hard enough to fear.",
            fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 25.sp, color = KshanaColors.Text,
        )
        tree?.causes?.firstOrNull()?.let { c ->
            Text("The strongest pressure on you now is ${c.title.lowercase()}, ${c.strengthWords.lowercase()}. ${c.untilWords.replaceFirstChar { it.uppercase() }}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        }
        Text(
            "Nothing in a chart needs an expensive pooja to be set right. Deeds, giving and patience do the real work. Be careful of anyone who asks for large sums to remove a 'dosha'.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Sky,
        )
        Door("What will help, free", onClick = { doors.remedy(tree?.life) })
        Door("How Kshana works it out", onClick = doors.how)
    }
}

/** A stretch of the past, said plainly, with yes and no. */
@Composable
private fun PastLine(c: PastCheckRecord, events: List<com.kshana.app.data.LifeEvent>, onAnswer: () -> Unit) {
    val context = LocalContext.current
    val from = Instants.toLocal(c.from).toLocalDate()
    val to = Instants.toLocal(c.to).toLocalDate()
    val confirmed = events.any { it.life == c.life && it.note.startsWith("Confirmed") && !it.date.isBefore(from) && !it.date.isAfter(to) }
    var answered by remember(c.from.taiSeconds) { mutableStateOf(confirmed) }
    Column(Modifier.fillMaxWidth().padding(vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Row(verticalAlignment = Alignment.Top) {
            Text(lifeGlyph(c.life), color = if (c.good) KshanaColors.Leaf else KshanaColors.Ember, modifier = Modifier.padding(end = 10.dp, top = 2.dp))
            Text(c.statement, style = MaterialTheme.typography.bodyLarge, color = KshanaColors.Text, modifier = Modifier.weight(1f))
        }
        if (answered) Text("✓ You confirmed this", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Leaf, modifier = Modifier.padding(start = 26.dp))
        else Row(Modifier.padding(start = 26.dp), horizontalArrangement = Arrangement.spacedBy(16.dp)) {
            Text("THAT HAPPENED", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable {
                val mid = from.plusDays(java.time.temporal.ChronoUnit.DAYS.between(from, to) / 2)
                Journal.add(context, mid, c.life, c.good, "Confirmed from the reading")
                answered = true
                onAnswer()
            }.padding(vertical = 4.dp))
            Text("NOT REALLY", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable { answered = true }.padding(vertical = 4.dp))
        }
    }
}

/** Choose what is on your mind, grouped the way people think of their troubles. */
@Composable
private fun SituationPicker(onDone: () -> Unit) {
    val context = LocalContext.current
    Column(Modifier.fillMaxWidth().glass(16.dp).padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("What are you going through?", fontFamily = FontFamily.Serif, fontSize = 18.sp, color = KshanaColors.Text)
        Text("Choose one or more. You can change this any time.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        for ((group, items) in Situations.ALL.groupBy { it.group }) {
            Text(group.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.padding(top = 6.dp))
            for (s in items) OptionBox(s.says, selected = Situations.has(s.key), onClick = { Situations.toggle(context, s.key) }, modifier = Modifier.fillMaxWidth())
        }
        Text("FEARS", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.padding(top = 6.dp))
        OptionBox("An astrologer said something is wrong in my chart", selected = Situations.has(Situations.CHART_FEAR), onClick = { Situations.toggle(context, Situations.CHART_FEAR) }, modifier = Modifier.fillMaxWidth())
        Spacer(Modifier.height(4.dp))
        GoldButton("DONE", onClick = onDone, modifier = Modifier.fillMaxWidth())
    }
}
