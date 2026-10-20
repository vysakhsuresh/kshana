package com.kshana.app.ui.screens

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
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
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Journal
import com.kshana.app.data.Lang
import com.kshana.app.data.PastReveal
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.BirthRefineRecord
import uniffi.kshana_ffi.PastAnswerInput
import uniffi.kshana_ffi.PastCheckRecord

/** The clause that matters most on this screen: a No is never wasted. */
private const val WRONG_HELPS = "A wrong guess still helps, because it narrows down the birth time."

/**
 * Your past, read first. Before it says a word about the future, Kshana tells the person
 * a few dated stretches of their own life, from their teens to now, and asks: Yes, Close
 * or No? The score is shown honestly; every answer sharpens the birth time; more
 * questions can be asked as long as the person likes; and one prediction about the
 * coming weeks is sealed, to be opened when it has passed.
 */
@Composable
fun PastCheckScreen(
    profile: BirthProfile,
    settings: EngineSettings,
    onDone: () -> Unit,
    onProfile: (BirthProfile) -> Unit = {},
    onRemedy: () -> Unit = {},
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val items = remember { mutableStateListOf<PastReveal.Item>().apply { addAll(PastReveal.answers(context)) } }
    var loading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var refine by remember { mutableStateOf<BirthRefineRecord?>(null) }
    var refining by remember { mutableStateOf(false) }
    var learnLine by remember { mutableStateOf<String?>(null) }
    var seal by remember { mutableStateOf(PastReveal.pending(context)) }
    var timeNote by remember { mutableStateOf<String?>(null) }
    val windowMin = (profile.uncertaintySeconds() / 60.0).coerceIn(15.0, 180.0)

    fun store() = PastReveal.saveAnswers(context, items.toList())

    fun fetch(count: Int, sharpen: Boolean) {
        if (loading) return
        loading = true
        error = null
        scope.launch {
            try {
                val engine = EngineProvider.get(context)
                val asked = items.toList()
                val got: List<PastCheckRecord> = withContext(Dispatchers.Default) {
                    engine.pastReveal(
                        profile.birthInstant(), profile.latitude, profile.longitude, settings.houseSystem, settings.ayanamsa,
                        Instants.now(), Lang.code, windowMin, asked.map { it.from() }, asked.map { it.to() }, count.toUInt(), sharpen,
                    )
                }
                for (c in got) items.add(PastReveal.Item(c.from.taiSeconds, c.from.nanos.toInt(), c.to.taiSeconds, c.to.nanos.toInt(), c.life, c.good, c.statement, ""))
                if (got.isEmpty()) error = "There are no more stretches strong enough to ask about. Your answers so far are what the birth time rests on."
                store()
            } catch (e: Exception) {
                error = e.message ?: e.toString()
            }
            loading = false
        }
    }

    fun runRefine() {
        val answered = items.filter { it.verdict in setOf("yes", "close", "no") }
        if (answered.size < 3) return
        refining = true
        scope.launch {
            runCatching {
                val engine = EngineProvider.get(context)
                refine = withContext(Dispatchers.Default) {
                    engine.refineBirthTime(
                        profile.birthInstant(), profile.latitude, profile.longitude, settings.houseSystem, settings.ayanamsa, windowMin,
                        answered.map { PastAnswerInput(it.from(), it.to(), it.life, it.good, it.verdict) },
                    )
                }
                // Which way of reading fits this person's own past best, said plainly.
                learnLine = withContext(Dispatchers.Default) {
                    engine.learnTechniques(
                        profile.birthInstant(), profile.latitude, profile.longitude, settings.houseSystem, settings.ayanamsa,
                        answered.map { PastAnswerInput(it.from(), it.to(), it.life, it.good, it.verdict) }, Lang.code,
                    ).plain
                }
            }
            refining = false
        }
    }

    LaunchedEffect(profile) { if (items.isEmpty()) fetch(5, false) else runRefine() }

    val shown = run {
        // One stretch at a time: every answered one, then the next to answer.
        val firstOpen = items.indexOfFirst { it.verdict.isEmpty() }
        if (firstOpen < 0) items.toList() else items.take(firstOpen + 1)
    }
    val allAnswered = items.isNotEmpty() && items.none { it.verdict.isEmpty() }

    // Seal one prediction the first time the reveal is finished, if none is waiting.
    LaunchedEffect(allAnswered) {
        if (!allAnswered || seal != null) return@LaunchedEffect
        runCatching {
            val engine = EngineProvider.get(context)
            val pick = withContext(Dispatchers.Default) {
                engine.sealPick(profile.birthInstant(), profile.latitude, profile.longitude, settings.houseSystem, settings.ayanamsa, Instants.now(), Lang.code)
            }
            if (pick != null) seal = PastReveal.seal(context, pick.from, pick.to, pick.life, pick.good, pick.statement, pick.question)
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 56.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("FIRST, YOUR PAST", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text("Let me tell you what has already happened.", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "Before I tell you anything about what is coming, here are a few stretches of your own life, read from your birth time alone. You tell me if I am right. $WRONG_HELPS",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )

        for ((i, item) in shown.withIndex()) {
            AnimatedVisibility(visible = true, enter = fadeIn() + expandVertically()) {
                StretchCard(item, onAnswer = { v ->
                    val first = items[i].verdict.isEmpty()
                    items[i] = item.copy(verdict = v)
                    store()
                    if (v == "yes" && first) {
                        val from = Instants.toLocal(item.from()).toLocalDate()
                        val to = Instants.toLocal(item.to()).toLocalDate()
                        Journal.add(context, from.plusDays(java.time.temporal.ChronoUnit.DAYS.between(from, to) / 2), item.life, item.good, "Confirmed when Kshana read my past")
                    }
                    if (items.none { it.verdict.isEmpty() }) runRefine()
                })
            }
        }
        if (loading) Working(if (items.isEmpty()) "reading your life from your birth time…" else "finding the stretches that will tell your birth time apart…")
        ErrorLine(error)

        if (allAnswered) {
            ScoreCard(items.toList())
            BirthTimeCard(
                profile, refine, refining, timeNote, learnLine,
                onUse = { fit ->
                    val local = Instants.toLocal(fit.birth, profile.zone())
                    val p = profile.copy(year = local.year, month = local.monthValue, day = local.dayOfMonth, hour = local.hour, minute = local.minute, second = local.second)
                    ProfileStore.save(context, p)
                    timeNote = "Your birth time is now ${"%02d:%02d".format(p.hour, p.minute)}. Every reading in the app uses it from now on."
                    refine = null
                    onProfile(p)
                },
            )
            QuietButton(
                if (loading) "FINDING MORE…" else "ASK ME MORE QUESTIONS", onClick = { fetch(3, true) }, modifier = Modifier.fillMaxWidth(),
            )
            Text(
                "Each question is chosen so that its answer tells nearby birth times apart. Answer as many as you like; the more you answer, the surer the birth time.",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
            )
            seal?.let { SealCard(it, onRemedy) }
        }

        GoldButton(if (allAnswered) "NOW, MY DAY" else if (items.any { it.verdict.isNotEmpty() }) "FINISH LATER" else "SKIP FOR NOW", onClick = onDone, modifier = Modifier.fillMaxWidth())
        Text("Your answers stay on this phone.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    }
}

@Composable
private fun StretchCard(item: PastReveal.Item, onAnswer: (String) -> Unit) {
    Column(Modifier.fillMaxWidth().glass(16.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Text("${lifeGlyph(item.life)} ${lifeName(item.life).uppercase()}", style = MaterialTheme.typography.labelSmall, color = if (item.good) KshanaColors.Leaf else KshanaColors.Ember)
        Text(item.statement, fontFamily = FontFamily.Serif, fontSize = 19.sp, lineHeight = 27.sp, color = KshanaColors.Text)
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            OptionBox("Yes", selected = item.verdict == "yes", onClick = { onAnswer("yes") }, modifier = Modifier.weight(1f))
            OptionBox("Close", selected = item.verdict == "close", onClick = { onAnswer("close") }, modifier = Modifier.weight(1f))
            OptionBox("No", selected = item.verdict == "no", onClick = { onAnswer("no") }, modifier = Modifier.weight(1f))
        }
        Text(
            if (item.verdict == "unsure") "NOT SURE ✓" else "NOT SURE",
            style = MaterialTheme.typography.labelSmall, color = if (item.verdict == "unsure") KshanaColors.Gold else KshanaColors.Muted,
            modifier = Modifier.clickable { onAnswer("unsure") }.padding(vertical = 4.dp),
        )
        when (item.verdict) {
            "no" -> Text("Not this one? That tells me your birth time is a little different. $WRONG_HELPS", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
            "close" -> Text("Close counts too: something like it, or near that time. It helps place your birth time.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
            "unsure" -> Text("That is fine. This one is left out of the score and the birth time.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
    }
}

@Composable
private fun ScoreCard(items: List<PastReveal.Item>) {
    val context = LocalContext.current
    val (right, close, asked) = PastReveal.score(items.filter { it.verdict != "unsure" })
    if (asked == 0) return
    val line = buildString {
        append("I read $right of $asked stretches of your past right")
        if (close > 0) append(", and $close close")
        append(".")
    }
    Column(Modifier.fillMaxWidth().glass(18.dp).padding(18.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("THE SCORE, HONESTLY", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(line, fontFamily = FontFamily.Serif, fontSize = 24.sp, lineHeight = 32.sp, color = KshanaColors.Gold)
        Text(
            if ((right + close) * 2 >= asked) "From your birth time alone. Now I can tell you about what is coming, and you know how far to trust it."
            else "Fewer than half matched, which usually means the birth time is a little off. $WRONG_HELPS Answer a few more below and I will find the time that fits your life.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
        )
        if (right + close > 0) {
            Text(
                "SHARE MY SCORE ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                modifier = Modifier.clickable {
                    com.kshana.app.system.ShareCard.share(
                        context, "MY PAST, READ BY KSHANA",
                        "Kshana read $right of $asked stretches of my past right${if (close > 0) ", and $close close" else ""}, from my birth time alone.",
                        "Kshana · your time, read plainly",
                    )
                }.padding(vertical = 6.dp),
            )
        }
    }
}

@Composable
private fun BirthTimeCard(profile: BirthProfile, r: BirthRefineRecord?, working: Boolean, note: String?, learnLine: String?, onUse: (uniffi.kshana_ffi.BirthFitRecord) -> Unit) {
    val recorded = "%02d:%02d".format(profile.hour, profile.minute)
    Column(Modifier.fillMaxWidth().glass(16.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("YOUR BIRTH TIME", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Sky)
        when {
            note != null -> Text(note, fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Text)
            working -> Working("checking every minute around $recorded against your answers…")
            r == null -> Text("Answer at least three stretches and I will check your recorded time, $recorded, against your life.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
            r.suggested != null -> {
                val s = r.suggested!!
                val t = Instants.toLocal(s.birth, profile.zone())
                val mins = kotlin.math.abs(s.shiftMin).toInt()
                Text(
                    "Your answers fit a birth time of ${"%02d:%02d".format(t.hour, t.minute)} better than $recorded: $mins minute${if (mins == 1) "" else "s"} ${if (s.shiftMin > 0) "later" else "earlier"}.",
                    fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Text,
                )
                Text(
                    "That time explains ${s.explained} of your answers; $recorded explains ${r.recordedExplains}. Recorded times are often rounded or noted a little late. You can always change it back under Birth details.",
                    style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
                )
                Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    GoldButton("USE ${"%02d:%02d".format(t.hour, t.minute)}", onClick = { onUse(s) }, modifier = Modifier.weight(1f))
                }
                Text("Not sure yet? Answer more questions below; the suggestion firms up or goes away.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            }
            else -> {
                val best = r.fits.firstOrNull()
                Text(
                    if (best == null || kotlin.math.abs(best.shiftMin) < 2.0) "Your answers agree with your recorded time, $recorded."
                    else "Your answers lean a little toward ${"%02d:%02d".format(Instants.toLocal(best.birth, profile.zone()).hour, Instants.toLocal(best.birth, profile.zone()).minute)}, but not clearly enough to change $recorded yet.",
                    fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Text,
                )
                Text("More answers make this surer. $WRONG_HELPS", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            }
        }
        // The stretch of birth times the answers point to, once they point anywhere firmly.
        if (r != null && note == null && !working) {
            val a = r.bracketFrom
            val b = r.bracketTo
            if (a != null && b != null && r.log10Bayes >= 1.0) {
                val f = { e: uniffi.kshana_ffi.ExactInstant -> Instants.toLocal(e, profile.zone()).let { "%02d:%02d".format(it.hour, it.minute) } }
                Text(
                    "Your answers place your birth between ${f(a)} and ${f(b)}: inside that stretch, everything they rest on holds.",
                    style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky,
                )
            }
        }
        if (!learnLine.isNullOrBlank() && note == null) Text(learnLine, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Leaf)
    }
}

/** A prediction written down now and locked until its window has passed. */
@Composable
internal fun SealCard(seal: PastReveal.Seal, onRemedy: () -> Unit, onAnswer: ((String) -> Unit)? = null) {
    val now = System.currentTimeMillis()
    val opens = java.time.Instant.ofEpochMilli(seal.opensAt()).atZone(java.time.ZoneId.systemDefault())
    val days = java.time.temporal.ChronoUnit.DAYS.between(java.time.LocalDate.now(), opens.toLocalDate())
    Column(Modifier.fillMaxWidth().glass(18.dp).padding(18.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("A SEALED PREDICTION", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        if (!seal.isOpen(now)) {
            Text(
                "I have written down one thing about the coming weeks and sealed it. It opens on ${opens.format(java.time.format.DateTimeFormatter.ofPattern("d MMM uuuu"))}${if (days > 0) ", in $days day${if (days == 1L) "" else "s"}" else ""}. Then you will see if I was right.",
                fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 24.sp, color = KshanaColors.Text,
            )
            Text(
                "Seal ${seal.shortCode()} · sealed ${java.time.Instant.ofEpochMilli(seal.sealedAt).atZone(java.time.ZoneId.systemDefault()).format(java.time.format.DateTimeFormatter.ofPattern("d MMM uuuu, HH:mm"))}. The code is a fingerprint of the words: if a single letter changed, it would not match.",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
            )
        } else {
            Text("Opened today. Sealed ${java.time.Instant.ofEpochMilli(seal.sealedAt).atZone(java.time.ZoneId.systemDefault()).format(java.time.format.DateTimeFormatter.ofPattern("d MMM uuuu"))}, seal ${seal.shortCode()}:", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            Text(seal.statement, fontFamily = FontFamily.Serif, fontSize = 19.sp, lineHeight = 27.sp, color = KshanaColors.Text)
            if (seal.verdict.isEmpty() && onAnswer != null) {
                Text("Did it happen?", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    OptionBox("Yes", selected = false, onClick = { onAnswer("yes") }, modifier = Modifier.weight(1f))
                    OptionBox("Close", selected = false, onClick = { onAnswer("close") }, modifier = Modifier.weight(1f))
                    OptionBox("No", selected = false, onClick = { onAnswer("no") }, modifier = Modifier.weight(1f))
                }
                Text(WRONG_HELPS, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            }
        }
        // Only once opened: before that, the hint would give away what it says.
        if (seal.isOpen(now) && !seal.good) {
            Text(
                "Whatever it says, nothing is fixed: What helps has the deeds that ease a hard stretch. WHAT HELPS ›",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onRemedy),
            )
        }
    }
}
