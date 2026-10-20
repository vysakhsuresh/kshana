package com.kshana.app.ui.screens

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
import uniffi.kshana_ffi.AnswerRecord
import uniffi.kshana_ffi.QuestionRecord
import uniffi.kshana_ffi.TopicRecord
import uniffi.kshana_ffi.questions
import uniffi.kshana_ffi.topics

/**
 * Ask: pick a life and a question, get a plain answer deduced from the birth chart, the
 * running periods, the sky ahead and the horary chart of the moment. No typing, no AI.
 */
@Composable
fun AskScreen(
    profile: BirthProfile,
    place: Place,
    settings: EngineSettings,
    initialLife: String? = null,
    onBack: (() -> Unit)? = null,
    onRemedy: (String) -> Unit = {},
    onGo: (uniffi.kshana_ffi.AskActionRecord) -> Unit = {},
) {
    val context = LocalContext.current
    val qs: List<QuestionRecord> = remember { questions() }
    val allTopics: List<TopicRecord> = remember { topics() }
    // Another screen may send the person here with a question already chosen.
    val pre = remember { Triple(AskPrefill.life, AskPrefill.topic, AskPrefill.question).also { AskPrefill.life = null; AskPrefill.topic = null; AskPrefill.question = null } }
    var life by remember { mutableStateOf(pre.first ?: initialLife ?: "Love") }
    var question by remember { mutableStateOf(pre.third ?: qs.firstOrNull()?.key ?: "will_it_happen") }
    // A topic of the dictionary narrows the life to one matter; null means the life itself.
    var topic by remember { mutableStateOf(allTopics.firstOrNull { it.key == pre.second }) }
    var showTopics by remember { mutableStateOf(false) }
    var answer by remember { mutableStateOf<AnswerRecord?>(null) }
    var asking by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var askedKey by remember { mutableStateOf(if (pre.first != null) "pre|${System.currentTimeMillis()}" else null) }
    var showWhy by remember { mutableStateOf(false) }

    LaunchedEffect(askedKey) {
        val key = askedKey ?: return@LaunchedEffect
        asking = true
        try {
            val engine = EngineProvider.get(context)
            val at = Instants.now()
            val t = topic
            answer = withContext(Dispatchers.Default) {
                if (t != null) {
                    engine.askTopic(
                        profile.birthInstant(), profile.latitude, profile.longitude,
                        place.latitude, place.longitude,
                        settings.houseSystem, settings.ayanamsa, t.key, question, at,
                    )
                } else {
                    engine.ask(
                        profile.birthInstant(), profile.latitude, profile.longitude,
                        place.latitude, place.longitude,
                        settings.houseSystem, settings.ayanamsa, life, question, at,
                    )
                }
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
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("ASK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            if (onBack != null) Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text(tr("Ask the sky"), style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text("Pick what it is about and what you want to know. You get a straight answer, when it happens, and what to do.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)

        // In your own words: typed, or spoken in English, Malayalam or Hindi. The engine
        // answers only what it understood; otherwise it says so and offers what it can.
        var free by remember { mutableStateOf("") }
        var reply by remember { mutableStateOf<uniffi.kshana_ffi.UnderstandingRecord?>(null) }
        fun run(a: uniffi.kshana_ffi.AskActionRecord) {
            if (a.kind == "ask") {
                life = a.life ?: life
                topic = allTopics.firstOrNull { it.key == a.topic }
                question = a.question ?: question
                answer = null
                askedKey = "${a.life}|${a.topic}|${a.question}|${System.currentTimeMillis()}"
            } else {
                onGo(a)
            }
        }
        fun understandIt(text: String) {
            val u = uniffi.kshana_ffi.understand(text)
            reply = u
            if (u.status == "answer") u.action?.let { run(it) } else answer = null
        }
        val speech = androidx.activity.compose.rememberLauncherForActivityResult(androidx.activity.result.contract.ActivityResultContracts.StartActivityForResult()) { r ->
            val said = r.data?.getStringArrayListExtra(android.speech.RecognizerIntent.EXTRA_RESULTS)?.firstOrNull()
            if (!said.isNullOrBlank()) {
                free = said
                understandIt(said)
            }
        }
        Text(
            "Kshana works offline. Every answer comes from your birth time and the sky, worked out on this phone, for a set list of questions about your life. When it is not sure what you asked, it says so and shows the closest questions it can answer; it never guesses.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            androidx.compose.material3.OutlinedTextField(
                value = free, onValueChange = { free = it },
                placeholder = { Text("Ask in your own words…", color = KshanaColors.Muted) },
                singleLine = false, maxLines = 3,
                colors = androidx.compose.material3.OutlinedTextFieldDefaults.colors(focusedBorderColor = KshanaColors.Gold, unfocusedBorderColor = KshanaColors.Line),
                modifier = Modifier.weight(1f),
            )
            Box(
                Modifier.size(52.dp).clip(CircleShape).border(1.dp, KshanaColors.Gold, CircleShape).clickable {
                    val i = android.content.Intent(android.speech.RecognizerIntent.ACTION_RECOGNIZE_SPEECH)
                        .putExtra(android.speech.RecognizerIntent.EXTRA_LANGUAGE_MODEL, android.speech.RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
                        .putExtra(android.speech.RecognizerIntent.EXTRA_LANGUAGE, when (com.kshana.app.data.Lang.code) { "ml" -> "ml-IN"; "hi" -> "hi-IN"; else -> "en-IN" })
                        .putExtra(android.speech.RecognizerIntent.EXTRA_PROMPT, "Ask your question")
                    runCatching { speech.launch(i) }
                },
                contentAlignment = Alignment.Center,
            ) { Text("🎙", fontSize = 20.sp) }
        }
        if (free.isNotBlank()) {
            QuietButton("ASK THIS", onClick = { understandIt(free) }, modifier = Modifier.fillMaxWidth())
        }
        reply?.let { u ->
            when (u.status) {
                "answer" -> Text(
                    "${u.message} Not what you meant? Change it below, or ask in other words.",
                    style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky,
                )
                else -> Column(
                    Modifier.fillMaxWidth().glass(16.dp).padding(14.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Text(
                        when (u.status) {
                            "care" -> "PLEASE READ THIS FIRST"
                            "outside" -> "THIS IS OUTSIDE WHAT KSHANA CAN ANSWER"
                            else -> "KSHANA IS NOT SURE IT UNDERSTOOD"
                        },
                        style = MaterialTheme.typography.labelSmall, color = if (u.status == "care") KshanaColors.Sky else KshanaColors.Gold,
                    )
                    Text(u.message, fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 23.sp, color = KshanaColors.Text)
                    if (u.status == "care") CareCard()
                    if (u.alternatives.isNotEmpty()) {
                        Text(if (u.status == "choose") "Is it one of these?" else "What Kshana can tell you instead:", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                        for (a in u.alternatives) OptionBox(a.label, selected = false, onClick = { reply = null; run(a.action) }, modifier = Modifier.fillMaxWidth())
                    }
                }
            }
        }

        SectionLabel(tr("About"))
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (l in LIFE_ORDER) Chip("${lifeGlyph(l)} ${lifeName(l).uppercase()}", selected = life == l, onClick = { life = l; topic = null; answer = null })
        }
        // The dictionary: the matters people actually ask about under this life.
        val lifeTopics = allTopics.filter { it.life == life }
        Text(
            if (showTopics) "EXACTLY WHAT? −" else "EXACTLY WHAT? ${lifeTopics.size} MATTERS UNDER ${life.uppercase()} +",
            style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
            modifier = Modifier.clickable { showTopics = !showTopics }.padding(vertical = 4.dp),
        )
        if (showTopics || topic != null) {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OptionBox("$life as a whole", selected = topic == null, onClick = { topic = null; answer = null })
                for (row in lifeTopics.chunked(2)) {
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        for (t in row) OptionBox(t.words, selected = topic?.key == t.key, onClick = { topic = t; answer = null }, modifier = Modifier.weight(1f))
                        if (row.size == 1) Spacer(Modifier.weight(1f))
                    }
                }
            }
        }
        SectionLabel(tr("Question"))
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            for (row in qs.chunked(2)) {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    for (q in row) {
                        OptionBox(tr(q.words), selected = question == q.key, onClick = { question = q.key; answer = null }, modifier = Modifier.weight(1f))
                    }
                    if (row.size == 1) Spacer(Modifier.weight(1f))
                }
            }
        }
        GoldButton(
            if (asking) "WORKING IT OUT…" else "ASK · ${(topic?.words ?: life).uppercase()}",
            enabled = !asking,
            onClick = { askedKey = "$life|${topic?.key}|$question|${System.currentTimeMillis()}" },
            modifier = Modifier.fillMaxWidth(),
        )
        ErrorLine(error)

        val a = answer
        if (a != null) {
            Spacer(Modifier.height(4.dp))
            Column(
                Modifier.fillMaxWidth().glass(18.dp).padding(18.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                    ScoreRing(a.score, size = 84.dp, stroke = 7.dp, label = "${a.confidence.toInt()}% sure")
                    Column(Modifier.weight(1f)) {
                        Text("${lifeGlyph(a.life)} ${topic?.words ?: a.life} · ${qs.firstOrNull { it.key == a.question }?.words ?: a.question}", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                        Text(a.verdict, style = MaterialTheme.typography.displayMedium, color = scoreColor(a.score))
                        Text(uniffi.kshana_ffi.sureWords(com.kshana.app.data.Lang.code, a.confidence), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                    }
                }
                Text(a.headline, fontFamily = FontFamily.Serif, fontSize = 20.sp, lineHeight = 28.sp, color = KshanaColors.Text)
                val ws = a.whenStart
                val we = a.whenEnd
                if (ws != null && we != null) {
                    HorizontalDivider(color = KshanaColors.LineSoft)
                    Text(a.whenWords.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                    Text(
                        if (sameDay(ws, we)) "${whenText(ws)} · ${hm(ws)} – ${hm(we)}" else "${whenText(ws)} – ${whenText(we)}",
                        fontFamily = FontFamily.Monospace, fontSize = 16.sp, color = KshanaColors.Sky,
                    )
                }
                val os = a.firstOpeningStart
                val oe = a.firstOpeningEnd
                if (os != null && oe != null) {
                    Text("The first real opening", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                    Text("${localText(os, WEEKDAY_DAY)} ${hm(os)} – ${if (sameDay(os, oe)) "" else localText(oe, WEEKDAY_DAY) + " "}${hm(oe)}", fontFamily = FontFamily.Monospace, fontSize = 13.sp, color = KshanaColors.Sky)
                }
                Row(horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                    Stat("today", a.now)
                    Stat("next 6 months", a.aheadMean)
                    Stat("at its best", a.aheadBest)
                }
            }

            Text(
                if (showWhy) "LESS −" else "WHAT LIES BEHIND IT +",
                style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                modifier = Modifier.clickable { showWhy = !showWhy }.padding(vertical = 4.dp),
            )
            if (showWhy) {
                for ((i, r) in a.reasons.withIndex()) {
                    HorizontalDivider(color = KshanaColors.LineSoft)
                    Row(Modifier.padding(vertical = 6.dp), verticalAlignment = Alignment.Top) {
                        Box(Modifier.padding(top = 6.dp).size(8.dp).clip(CircleShape).background(KshanaColors.Sky))
                        Spacer(Modifier.width(10.dp))
                        Column(Modifier.weight(1f)) {
                            Text(r.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                        }
                    }
                }
                if (a.shifts.isNotEmpty()) {
                    SectionLabel("What changes in the days ahead")
                    for (s in a.shifts) {
                        Row(Modifier.fillMaxWidth().padding(vertical = 2.dp)) {
                            Text(localText(s.at, WEEKDAY_DAY), fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky, modifier = Modifier.width(92.dp))
                            Text(s.what, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
                        }
                    }
                }
            }
            // A low score, or any answer that says wait or no, comes with what to do meanwhile.
            if (a.score < 56 || a.verdict in HOLD_VERDICTS) {
                QuietButton("WHAT TO DO FOR ${lifeName(a.life).uppercase()}", onClick = { onRemedy(a.life) }, modifier = Modifier.fillMaxWidth())
            }
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                QuietButton("READ IT ALOUD", onClick = { com.kshana.app.system.Speak.say(context, "${a.verdict}. ${a.headline}", com.kshana.app.data.Lang.code) }, modifier = Modifier.weight(1f))
                QuietButton("SHARE", onClick = {
                    com.kshana.app.system.ShareCard.share(context, "${lifeName(a.life)} · ${qs.firstOrNull { it.key == a.question }?.words ?: ""}", a.headline, "Asked ${localText(Instants.now(), DATE_ONLY)}")
                }, modifier = Modifier.weight(1f))
            }
            LaunchedEffect(a) { com.kshana.app.data.Care.note(context, a.life, a.question) }
            if (com.kshana.app.data.Care.heavy(context, a.life, a.question, a.score)) CareCard()
            Text("Asked ${localText(Instants.now(), DATE_TIME_S)} at ${place.name}. Worked out on this phone; nothing leaves it.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
    }
}

/** The engine's wait-or-no verdicts (ask.rs); each is shown with the remedy button. */
private val HOLD_VERDICTS = setOf(
    "Hold", "Not yet", "Not in this phase", "Not in this period", "Not as it was",
    "Not today", "Not this minute", "Wait for the hour", "Not within two years", "Beyond a year",
)

@Composable
private fun Stat(label: String, v: Double) {
    Column {
        Text(label.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        Text(v.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 18.sp, color = scoreColor(v))
    }
}

private fun sameDay(a: uniffi.kshana_ffi.ExactInstant, b: uniffi.kshana_ffi.ExactInstant): Boolean =
    Instants.toLocal(a).toLocalDate() == Instants.toLocal(b).toLocalDate()
