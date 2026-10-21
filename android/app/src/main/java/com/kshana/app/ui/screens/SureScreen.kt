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
import com.kshana.app.data.Lang
import com.kshana.app.data.Place
import com.kshana.app.data.PlaceClock
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.SureHoursRecord
import uniffi.kshana_ffi.SureWindowRecord

/**
 * The windows already read today, so going back and forth costs nothing. Keyed by the
 * person, the place, the language and the day: a new day reads again.
 */
private object SureCache {
    var key: String? = null
    var windows: List<SureWindowRecord>? = null
    var nextKey: String? = null
    var next: SureWindowRecord? = null
    var nextRead = false

    fun keyOf(profile: BirthProfile, place: Place, settings: EngineSettings): String =
        listOf(profile.birthInstant().taiSeconds, profile.latitude, profile.longitude, place.latitude, place.longitude, settings.ayanamsa, settings.houseSystem, Lang.code, java.time.LocalDate.now()).joinToString("|")
}

/**
 * What's coming, in dates. The windows of the next two years where the independent ways
 * of reading the birth chart agree: the strong ones first, each with its big date, the
 * sentence (more specific the more the signs agree), how sure it is, the likely days
 * and, when the narrowing is sharp, the hours. A hard stretch always comes with kind
 * words and what helps. Only the strong ones are said firmly; nothing is said to be fixed.
 */
@Composable
fun SureScreen(profile: BirthProfile, place: Place, settings: EngineSettings, onBack: () -> Unit, onRemedy: () -> Unit) {
    val context = LocalContext.current
    val key = SureCache.keyOf(profile, place, settings)
    var list by remember(key) { mutableStateOf(if (SureCache.key == key) SureCache.windows else null) }
    var error by remember { mutableStateOf<String?>(null) }
    var showFaint by remember { mutableStateOf(false) }
    val expert = com.kshana.app.data.Expert.on(context)
    LaunchedEffect(key) {
        if (list != null) return@LaunchedEffect
        runCatching {
            val engine = EngineProvider.get(context)
            val got = withContext(Dispatchers.Default) {
                // Each way of reading's say follows its own record for this person, once earned.
                engine.sureWindowsTrusted(
                    profile.birthInstant(), profile.latitude, profile.longitude, settings.houseSystem, settings.ayanamsa,
                    place.latitude, place.longitude, PlaceClock.offsetMinutes(place), Instants.now(), 24u, emptyList(), 30u, Lang.code,
                    com.kshana.app.data.Integrity.systemVotes(context, profile),
                )
            }
            SureCache.key = key
            SureCache.windows = got
            list = got
            // The firm ones are sealed before they come, so the score can count them.
            runCatching { com.kshana.app.data.Trust.sealSureWindows(context, profile, got) }
        }.onFailure { error = it.message ?: it.toString() }
    }
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("‹ BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onBack).padding(vertical = 4.dp))
        Text(tr("WHAT'S COMING, IN DATES"), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(tr("When the signs agree"), style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            tr("Nine different ways of reading your birth are laid side by side. Where most of them point to the same days, the words get specific: what, who and when. Where only a few do, they stay general."),
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        ErrorLine(error)
        val l = list ?: run { if (error == null) Working(tr("laying the nine readings side by side…")); return@Column }
        val strong = l.filter { it.tier == "strong" }.sortedBy { it.start.taiSeconds }
        val likely = l.filter { it.tier == "likely" }.sortedBy { it.start.taiSeconds }
        val faint = l.filter { it.tier == "possible" }.sortedBy { it.start.taiSeconds }
        if (strong.isNotEmpty()) SectionLabel(tr("The signs agree strongly"))
        for (w in strong) SureCard(w, profile, place, settings, expert, onRemedy)
        if (likely.isNotEmpty()) SectionLabel(tr("Likely"))
        for (w in likely) SureCard(w, profile, place, settings, expert, onRemedy)
        if (strong.isEmpty() && likely.isEmpty()) {
            Text(
                tr("No stretch in the next two years has most of the signs pointing the same way. That is common: life runs on its own steady course, and the fainter signs are below."),
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
            )
        }
        if (faint.isNotEmpty()) {
            Door(if (showFaint) tr("Hide the fainter signs") else "${tr("The fainter signs")} (${faint.size})", onClick = { showFaint = !showFaint })
            AnimatedVisibility(showFaint, enter = fadeIn() + expandVertically()) {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    for (w in faint) SureCard(w, profile, place, settings, expert, onRemedy)
                }
            }
        }
        Text(
            tr("Firm words only where the signs agree strongly, and even then a reading is a strong lean, never a promise. A hard stretch passes, and what you do in it counts."),
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(top = 8.dp),
        )
    }
}

/** One window: the big date, the sentence, how sure, the days, and what helps. */
@Composable
private fun SureCard(w: SureWindowRecord, profile: BirthProfile, place: Place, settings: EngineSettings, expert: Boolean, onRemedy: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var open by remember { mutableStateOf(false) }
    var hours by remember { mutableStateOf<SureHoursRecord?>(null) }
    var narrowing by remember { mutableStateOf(false) }
    val strong = w.tier == "strong"
    val tone = if (w.isHard) KshanaColors.Ember else KshanaColors.Leaf
    Column(Modifier.fillMaxWidth().glass(if (strong) 18.dp else 16.dp).padding(if (strong) 18.dp else 16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row {
            Text(
                (listOf("${lifeGlyph(w.area)} ${w.areaName.uppercase()}") + w.alsoNames.take(2).map { it.uppercase() } + listOfNotNull((w.alsoNames.size - 2).takeIf { it > 0 }?.let { "+$it MORE" })).joinToString(" · "),
                style = MaterialTheme.typography.labelSmall, color = tone, modifier = Modifier.weight(1f),
            )
            Text("●".repeat(w.agree.toInt().coerceIn(1, 9)) + "○".repeat((w.of.toInt() - w.agree.toInt()).coerceIn(0, 9)), fontSize = 9.sp, color = if (strong) KshanaColors.Gold else KshanaColors.Muted)
        }
        Text(w.dateLine, fontFamily = FontFamily.Serif, fontSize = if (strong) 28.sp else 22.sp, lineHeight = if (strong) 34.sp else 28.sp, color = if (strong) KshanaColors.Gold else KshanaColors.Text)
        Text(w.headline, fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 26.sp, color = KshanaColors.Text)
        Text(w.sureLine, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
        // How sure, as a number only once 20 like it have been checked; words before that.
        if (strong || w.tier == "likely") {
            val conf = remember(w.tier) { com.kshana.app.data.Integrity.confidence(context, profile, w.tier) }
            conf?.let { Text(it.words, style = MaterialTheme.typography.bodySmall, color = if (it.numberShown) KshanaColors.Text else KshanaColors.Muted) }
        }
        if (w.bestLine.isNotBlank()) Text(w.bestLine, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Gold)
        // A likely window can be narrowed further on request; the strong ones already are.
        val h = hours
        if (h != null) {
            Text(
                if (h.hours != null) h.line else tr("No single hour stands out; the days above are as close as the signs go."),
                style = MaterialTheme.typography.bodyMedium, color = if (h.hours != null) KshanaColors.Gold else KshanaColors.Muted,
            )
        } else if (w.tier == "likely" && w.bestHours == null && w.bestDays.isNotEmpty()) {
            Text(
                if (narrowing) tr("NARROWING…") else tr("NARROW TO THE HOUR ›"),
                style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                modifier = Modifier.clickable(enabled = !narrowing) {
                    narrowing = true
                    scope.launch {
                        runCatching {
                            val engine = EngineProvider.get(context)
                            hours = withContext(Dispatchers.Default) {
                                engine.sureWindowHours(
                                    profile.birthInstant(), profile.latitude, profile.longitude, settings.houseSystem, settings.ayanamsa,
                                    place.latitude, place.longitude, PlaceClock.offsetMinutes(place), w.area, w.isHard, w.start, w.end, Instants.now(),
                                    w.peak, w.systemKeys, Lang.code,
                                )
                            }
                        }
                        narrowing = false
                    }
                }.padding(vertical = 4.dp),
            )
        }
        Text(
            if (open) tr("LESS ‹") else tr("WHY ›"),
            style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
            modifier = Modifier.clickable { open = !open }.padding(vertical = 2.dp),
        )
        AnimatedVisibility(open, enter = fadeIn() + expandVertically()) {
            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text(w.detail, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                if (expert) Text(w.expert, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            }
        }
        if (w.isHard) {
            w.remedyHint?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Leaf) }
            Text(tr("WHAT HELPS ›"), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onRemedy).padding(vertical = 4.dp))
        }
        WhereFrom(w.sourceKeys)
    }
}

/**
 * The single next strong window, for the Now tab: its date, its sentence and how sure
 * it is, with a door to every date. Shows nothing while it reads or when no stretch of
 * the coming year has the signs agreeing.
 */
@Composable
fun SureNextCard(profile: BirthProfile, onOpen: () -> Unit) {
    val context = LocalContext.current
    val settings = remember { ProfileStore.settings(context) }
    val place = remember { ProfileStore.currentLocation(context) }
    val key = SureCache.keyOf(profile, place, settings)
    var next by remember(key) { mutableStateOf(if (SureCache.nextKey == key) SureCache.next else null) }
    var read by remember(key) { mutableStateOf(SureCache.nextKey == key && SureCache.nextRead) }
    LaunchedEffect(key) {
        if (read) return@LaunchedEffect
        runCatching {
            val engine = EngineProvider.get(context)
            val got = withContext(Dispatchers.Default) {
                engine.sureNext(
                    profile.birthInstant(), profile.latitude, profile.longitude, settings.houseSystem, settings.ayanamsa,
                    place.latitude, place.longitude, PlaceClock.offsetMinutes(place), Instants.now(), 12u, Lang.code,
                )
            }
            SureCache.nextKey = key
            SureCache.next = got
            SureCache.nextRead = true
            next = got
            read = true
        }
    }
    val w = next ?: return
    val strong = w.tier == "strong"
    Column(
        Modifier.fillMaxWidth().glass(16.dp).clickable(onClick = onOpen).padding(14.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Text(
            "${tr("NEXT, IN DATES")} · ${lifeGlyph(w.area)} ${w.areaName.uppercase()}",
            style = MaterialTheme.typography.labelSmall, color = if (w.isHard) KshanaColors.Ember else KshanaColors.Gold,
        )
        Text(w.dateLine, fontFamily = FontFamily.Serif, fontSize = 22.sp, lineHeight = 28.sp, color = if (strong) KshanaColors.Gold else KshanaColors.Text)
        Text(w.headline, fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 22.sp, color = KshanaColors.Text)
        Text(w.sureLine, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
        if (w.bestLine.isNotBlank()) Text(w.bestLine, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Gold)
        if (w.isHard) w.remedyHint?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Leaf) }
        Text(tr("SEE EVERY DATE ›"), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.padding(top = 2.dp))
    }
}
