package com.kshana.app.ui.screens

import android.content.Intent
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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
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
import com.kshana.app.data.ClaimStore
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Lang
import com.kshana.app.data.PersonalWeights
import com.kshana.app.data.Trust
import com.kshana.app.engine.Instants
import com.kshana.app.ui.sky.KshanaFonts
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import com.kshana.app.data.Integrity
import uniffi.kshana_ffi.BlindResolveRecord
import uniffi.kshana_ffi.ClaimRecord
import uniffi.kshana_ffi.ScoreboardRecord
import uniffi.kshana_ffi.blindAnswers
import uniffi.kshana_ffi.blindJudge
import uniffi.kshana_ffi.blindQuestion
import uniffi.kshana_ffi.forecastTilt
import uniffi.kshana_ffi.scoreboard

/** The fingerprint as the person reads it: the first eight characters, in two fours. */
private fun ClaimRecord.code(): String = id.take(8).uppercase().chunked(4).joinToString("-")

/** The scoreboard over everything answered, computed on the phone. */
private fun scoreOf(context: android.content.Context, profile: BirthProfile): ScoreboardRecord =
    scoreboard(Trust.scoreItems(context, profile), Lang.code)

/**
 * The question asked before the sealed words are shown, written from the part of life
 * and the dates alone, so it says nothing of which way the prediction leaned.
 */
private fun blindQuestionOf(c: ClaimRecord, lon: Double): String =
    runCatching { blindQuestion(Lang.code, c.area, c.windowStart, c.windowEnd, lon) }.getOrNull()?.takeIf { it.isNotBlank() }
        ?: "Did anything happen between ${localText(c.windowStart, DATE_ONLY)} and ${localText(c.windowEnd, DATE_ONLY)}? Good, hard, or nothing much?"

/**
 * The plain scoreboard: one honest number ("For you, Kshana has been right 38 times out
 * of 45"), what guessing at random would have scored, and the sealed predictions behind
 * it, each with its fingerprint and the day it was sealed.
 */
@Composable
fun ScoreboardScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit, onOpen: (String) -> Unit = {}) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var version by remember { mutableIntStateOf(0) }
    var sealing by remember { mutableStateOf(false) }
    var sealNote by remember { mutableStateOf<String?>(null) }
    val nowTai = remember(version) { Instants.now().taiSeconds }
    val board = remember(version) { runCatching { scoreOf(context, profile) }.getOrNull() }
    val claims = remember(version) { ClaimStore.entries(context, profile.name) }
    val due = claims.filter { it.outcome.isEmpty() && it.windowEnd.taiSeconds <= nowTai && !Trust.isUnsure(context, profile, it.id) }
    val waiting = claims.filter { it.outcome.isEmpty() && it.windowEnd.taiSeconds > nowTai }
    val answered = claims.filter { it.outcome.isNotEmpty() }.sortedByDescending { it.resolvedAt?.taiSeconds ?: 0L }
    val tilt = remember { runCatching { forecastTilt(PersonalWeights.input(context, profile.name), Lang.code) }.getOrNull() }
    val answeredHere = remember { mutableStateListOf<ClaimRecord>() }
    val confidences = remember(version) { listOf("strong", "likely").mapNotNull { t -> Integrity.confidence(context, profile, t)?.let { t to it } } }
    val rates = remember(version) { Integrity.systemRates(context, profile) }

    LaunchedEffect(profile) {
        if (Trust.topUp(context, profile, settings) > 0) version++
    }

    fun share(b: ScoreboardRecord) {
        val send = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_SUBJECT, "My score with Kshana")
            putExtra(Intent.EXTRA_TEXT, "${b.shareText}\n\nKshana · your time, read plainly")
        }
        runCatching { context.startActivity(Intent.createChooser(send, "Share my score")) }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("YOUR SCORE", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("How right has it been?", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)

        Column(Modifier.fillMaxWidth().glass(18.dp).padding(18.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("THE SCORE, HONESTLY", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            if (board == null) {
                Working("adding up your answers…")
            } else {
                Text(board.headline, fontFamily = FontFamily.Serif, fontSize = if (board.total > 0u) 24.sp else 18.sp, lineHeight = if (board.total > 0u) 32.sp else 26.sp, color = if (board.total > 0u) KshanaColors.Gold else KshanaColors.Text)
                if (board.chanceLine.isNotEmpty()) Text(board.chanceLine, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                if (board.note.isNotEmpty()) Text(board.note, style = MaterialTheme.typography.bodySmall, color = if (board.beatsChance) KshanaColors.Leaf else KshanaColors.Sky)
                if (board.total > 0u) {
                    Text("SHARE MY SCORE ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable { share(board) }.padding(vertical = 6.dp))
                }
            }
        }

        if (board != null && board.lines.isNotEmpty()) {
            SectionLabel("Where the score comes from")
            for (l in board.lines) {
                HorizontalDivider(color = KshanaColors.LineSoft)
                Row(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text(l.words, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
                    Text(
                        if (l.right > l.chance + 0.25) "▲" else "·", fontFamily = KshanaFonts.mono, fontSize = 12.sp,
                        color = if (l.right > l.chance + 0.25) KshanaColors.Leaf else KshanaColors.Muted, modifier = Modifier.padding(start = 12.dp),
                    )
                }
            }
        }

        // The past, on its own line: it tuned the birth time, so it never counts.
        if (board != null && board.fitTotal > 0u) {
            SectionLabel("How well the reading fits your past")
            Text(board.fitLine, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
            if (board.fitLines.size > 1) {
                for (l in board.fitLines) Text("· ${l.words}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            }
        }

        // Cards answered here stay on screen, so the sealed words revealed after a blind answer can be read.
        val dueShown = (answeredHere + due).distinctBy { it.id }.sortedBy { it.windowStart.taiSeconds }
        if (dueShown.isNotEmpty()) {
            SectionLabel("Waiting for your answer")
            for (c in dueShown) {
                key(c.id) {
                    ClaimAnswerCard(c, profile, onAnswered = { if (answeredHere.none { it.id == c.id }) answeredHere.add(c); version++ }, onRemedy = { onOpen("parihara") })
                }
            }
        }

        SectionLabel("Sealed, still to come")
        if (waiting.isEmpty()) {
            Text("Nothing sealed is waiting right now. The app seals the firm dates of your forecast before they begin, a few at a time.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        } else {
            for (c in waiting) ClaimRow(c, nowTai)
        }
        sealNote?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky) }
        QuietButton(if (sealing) "SEALING…" else "SEAL WHAT IS COMING", onClick = {
            if (!sealing) scope.launch {
                sealing = true
                val n = Trust.topUp(context, profile, settings, force = true)
                sealNote = if (n > 0) "Sealed $n more. Each has its fingerprint below." else "Nothing new is firm enough to seal yet; the next look is in a week."
                version++
                sealing = false
            }
        }, modifier = Modifier.fillMaxWidth())

        if (answered.isNotEmpty()) {
            SectionLabel("Answered")
            for (c in answered) ClaimRow(c, nowTai)
        }

        tilt?.let {
            SectionLabel("What your answers do")
            Text(it.plain, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        }

        if (confidences.isNotEmpty()) {
            SectionLabel("How sure, in numbers only when earned")
            for ((t, c) in confidences) {
                Text(
                    "${if (t == "strong") "The firmest predictions" else "The likely ones"}: ${c.words}",
                    style = MaterialTheme.typography.bodyMedium, color = if (c.numberShown) KshanaColors.Text else KshanaColors.TextSoft,
                )
            }
        }

        if (rates.isNotEmpty()) {
            SectionLabel("Each way of reading, for you")
            for (r in rates) Text(r.words, style = MaterialTheme.typography.bodySmall, color = if (r.weighted) KshanaColors.Text else KshanaColors.Muted)
            Text(
                "Each one is checked only on sealed predictions it agreed with, after their dates. Below 10 checks it changes nothing; after that it counts a little more or less for you, and never so much that one way of reading outweighs the rest.",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
            )
        }

        SectionLabel("How this works")
        board?.let { Text(it.howItWorks, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft) }

        SectionLabel("How sealing works")
        Text(
            "Each prediction is written down before its dates begin, with a fingerprint of its exact words and the moment it was sealed. If a single letter changed afterwards, the fingerprint would no longer match, and it would show here. Nothing is sealed once its dates have begun.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
        )
        Spacer(Modifier.height(12.dp))
        Text(
            "A miss is shown as plainly as a hit. That is the only way the number means anything.",
            fontFamily = KshanaFonts.serif, fontStyle = FontStyle.Italic, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Text,
        )
    }
}

/** One sealed prediction: its words, its dates, when it was sealed, its fingerprint and how it turned out. */
@Composable
private fun ClaimRow(c: ClaimRecord, nowTai: Long) {
    HorizontalDivider(color = KshanaColors.LineSoft)
    Column(Modifier.fillMaxWidth().padding(vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text("${lifeGlyph(c.area)} ${lifeName(c.area).uppercase()}", style = MaterialTheme.typography.labelSmall, color = if (c.good) KshanaColors.Leaf else KshanaColors.Ember)
        Text(c.statement, fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 22.sp, color = KshanaColors.Text)
        val result = when (c.outcome) {
            "yes" -> "It came as sealed."
            "close" -> "Close: counted half."
            "no" -> "It did not come: counted as a miss."
            else -> if (c.windowStart.taiSeconds > nowTai) "Opens ${localText(c.windowStart, DATE_ONLY)}." else "Running until ${localText(c.windowEnd, DATE_ONLY)}."
        }
        Text(
            result, style = MaterialTheme.typography.bodySmall,
            color = when (c.outcome) { "yes" -> KshanaColors.Leaf; "close" -> KshanaColors.Gold; "no" -> KshanaColors.Sky; else -> KshanaColors.TextSoft },
        )
        Text(
            "Sealed ${localText(c.createdAt, DATE_ONLY)} · ${c.code()} · ${if (c.verified) "fingerprint matches" else "FINGERPRINT DOES NOT MATCH"}",
            fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = if (c.verified) KshanaColors.Muted else KshanaColors.Ember,
        )
    }
}

/**
 * The blind question for a closed window: first "did anything happen around money
 * between these dates, good, hard, or nothing much?", asked without the sealed words;
 * only after the answer is the sealed statement revealed, with how it turned out and a
 * kind word back. Blind by design: the answer cannot lean on what was predicted.
 */
@Composable
private fun ClaimAnswerCard(c: ClaimRecord, profile: BirthProfile, onAnswered: () -> Unit, onRemedy: () -> Unit, onScore: (() -> Unit)? = null) {
    val context = LocalContext.current
    var said by remember(c.id) { mutableStateOf<BlindResolveRecord?>(null) }
    var saidKey by remember(c.id) { mutableStateOf("") }
    val question = remember(c.id) { blindQuestionOf(c, profile.longitude) }
    val buttons = remember { runCatching { blindAnswers(Lang.code) }.getOrDefault(emptyList()) }
    Column(Modifier.fillMaxWidth().glass(16.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("A SEALED PREDICTION HAS CLOSED", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        val s = said
        if (s == null) {
            Text(question, fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 25.sp, color = KshanaColors.Text)
            Text(
                "Sealed ${localText(c.createdAt, DATE_ONLY)}, before it began · ${c.code()}. What it said stays hidden until you answer, so your answer is your own.",
                fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted,
            )
            val main = buttons.filter { it.key != "unsure" }
            for (row in main.chunked(2)) {
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    for (b in row) {
                        OptionBox(b.label, selected = false, onClick = {
                            Integrity.answerBlind(context, profile, c, b.key)?.let { said = it; saidKey = b.key; onAnswered() }
                        }, modifier = Modifier.weight(1f))
                    }
                    if (row.size == 1) Spacer(Modifier.weight(1f))
                }
            }
            buttons.firstOrNull { it.key == "unsure" }?.let { b ->
                Text(b.label.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable {
                    Integrity.answerBlind(context, profile, c, b.key)?.let { said = it; saidKey = b.key; onAnswered() }
                }.padding(vertical = 4.dp))
            }
        } else {
            Text(s.reveal, fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 23.sp, color = KshanaColors.Text)
            Text("Sealed ${localText(c.createdAt, DATE_ONLY)} · ${c.code()} · ${if (c.verified) "fingerprint matches" else "FINGERPRINT DOES NOT MATCH"}", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = if (c.verified) KshanaColors.Muted else KshanaColors.Ember)
            Row(horizontalArrangement = Arrangement.spacedBy(18.dp)) {
                if (!c.good || saidKey == "hard" || saidKey == "mixed") {
                    Text("WHAT HELPS ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onRemedy).padding(vertical = 4.dp))
                }
                if (onScore != null) {
                    Text("YOUR SCORE ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onScore).padding(vertical = 4.dp))
                }
            }
        }
    }
}

/**
 * The same blind question for the stretch sealed on the past-check page (a
 * [com.kshana.app.data.PastReveal.Seal]): asked first, then the sealed words revealed.
 * `onAnswered` gets the outcome key ("yes", "close", "no", or "unsure") for
 * `PastReveal.resolve`. For the Now tab's sealed-prediction card.
 */
@Composable
fun BlindSealCard(seal: com.kshana.app.data.PastReveal.Seal, lon: Double, onAnswered: (String) -> Unit, onRemedy: () -> Unit) {
    var said by remember(seal.fingerprint) { mutableStateOf<BlindResolveRecord?>(null) }
    var saidKey by remember(seal.fingerprint) { mutableStateOf("") }
    val buttons = remember { runCatching { blindAnswers(Lang.code) }.getOrDefault(emptyList()) }
    val question = remember(seal.fingerprint) {
        runCatching { blindQuestion(Lang.code, seal.life, uniffi.kshana_ffi.ExactInstant(seal.fromSec, 0u), uniffi.kshana_ffi.ExactInstant(seal.toSec, 0u), lon) }.getOrNull()?.takeIf { it.isNotBlank() } ?: seal.question
    }
    val sealedAt = remember(seal.fingerprint) { Instants.fromInstant(java.time.Instant.ofEpochMilli(seal.sealedAt)) }
    Column(Modifier.fillMaxWidth().glass(16.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("A SEALED PREDICTION HAS CLOSED", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        val s = said
        if (s == null) {
            Text(question, fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 25.sp, color = KshanaColors.Text)
            Text("Sealed before it began · ${seal.shortCode()}. What it said stays hidden until you answer.", fontFamily = KshanaFonts.mono, fontSize = 10.sp, color = KshanaColors.Muted)
            for (row in buttons.chunked(2)) {
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    for (b in row) {
                        OptionBox(b.label, selected = false, onClick = {
                            runCatching { blindJudge(seal.good, b.key, sealedAt, seal.statement, Lang.code, lon) }.getOrNull()?.let {
                                said = it
                                saidKey = b.key
                                onAnswered(it.outcome)
                            }
                        }, modifier = Modifier.weight(1f))
                    }
                    if (row.size == 1) Spacer(Modifier.weight(1f))
                }
            }
        } else {
            Text(s.reveal, fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 23.sp, color = KshanaColors.Text)
            if (!seal.good || saidKey == "hard" || saidKey == "mixed") {
                Text("WHAT HELPS ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onRemedy).padding(vertical = 4.dp))
            }
        }
    }
}

/**
 * For the Now tab: once a sealed prediction's window has closed, one gentle yes/no
 * ("Between 14 Nov and 19 Nov, did money come in well…?"). It also keeps a few
 * predictions sealed, topping up once a week. Shows nothing when nothing is due.
 * `onOpen` takes a route: "parihara" for what helps, "scoreboard" for the score.
 */
@Composable
fun ResolveDueCard(profile: BirthProfile, onOpen: (String) -> Unit) {
    val context = LocalContext.current
    var version by remember { mutableIntStateOf(0) }
    var current by remember { mutableStateOf<ClaimRecord?>(null) }
    LaunchedEffect(profile) {
        if (Trust.topUpDue(context, profile)) {
            withContext(Dispatchers.Default) { Trust.topUp(context, profile) }
        }
        version++
    }
    val next = remember(version) {
        ClaimStore.due(context, profile.name, Instants.now().taiSeconds).firstOrNull { !Trust.isUnsure(context, profile, it.id) }
    }
    val shown = current ?: next ?: return
    ClaimAnswerCard(
        shown,
        profile,
        onAnswered = { current = shown },
        onRemedy = { onOpen("parihara") },
        onScore = { onOpen("scoreboard") },
    )
    if (current != null && next != null && next.id != current?.id) {
        Text("NEXT ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable { current = null; version++ }.padding(vertical = 4.dp))
    }
}

/**
 * A small chip for the header of Now: "38 of 45 right", or "YOUR SCORE" before anything
 * is answered. Tapping it opens the scoreboard.
 */
@Composable
fun ScoreChip(profile: BirthProfile, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val board = remember(profile.name) { runCatching { scoreOf(context, profile) }.getOrNull() }
    val shape = RoundedCornerShape(14.dp)
    Box(
        modifier
            .height(28.dp)
            .clip(shape)
            .background(KshanaColors.Surface.copy(alpha = 0.6f))
            .border(1.dp, KshanaColors.Gold.copy(alpha = 0.45f), shape)
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp),
        contentAlignment = Alignment.Center,
    ) {
        val t = board?.takeIf { it.total > 0u }
        Text(
            if (t == null) "YOUR SCORE" else "${t.rightText} OF ${t.total} RIGHT",
            fontFamily = KshanaFonts.mono, fontSize = 10.sp, letterSpacing = 1.sp,
            color = if (t != null && !t.beatsChance) KshanaColors.Sky else KshanaColors.Gold,
        )
    }
}
