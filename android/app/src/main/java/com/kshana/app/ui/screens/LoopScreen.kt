package com.kshana.app.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Icon
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
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.halo
import com.kshana.app.ui.theme.glass
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberDatePickerState
import com.kshana.app.engine.EngineProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.BriefingRecord
import uniffi.kshana_ffi.HourRecord
import uniffi.kshana_ffi.LifeScoreRecord
import uniffi.kshana_ffi.MomentRecord
import uniffi.kshana_ffi.ShiftRecord
import uniffi.kshana_ffi.WindowRecord
import java.time.Duration
import java.time.Instant
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter

private val CLOCK: DateTimeFormatter = DateTimeFormatter.ofPattern("HH:mm:ss")
private val LONG_DATE: DateTimeFormatter = DateTimeFormatter.ofPattern("EEEE, d MMMM")

/**
 * The time loop: what this moment holds, a countdown to the next shift, nine lives,
 * the day's hours and windows, and what is coming. Recomputes at every shift.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LoopScreen(
    profile: BirthProfile,
    place: Place,
    settings: EngineSettings,
    onLife: (String) -> Unit,
    onAsk: () -> Unit,
    onRemedy: () -> Unit,
    onSettings: () -> Unit,
    onPlan: () -> Unit = {},
    onBook: () -> Unit = {},
    onTogether: () -> Unit = {},
    onWall: () -> Unit = {},
    onLetter: () -> Unit = {},
    onHindsight: () -> Unit = {},
    onRecord: () -> Unit = {},
    onAhead: () -> Unit = {},
) {
    val context = LocalContext.current
    var moment by remember { mutableStateOf<MomentRecord?>(null) }
    var sky by remember { mutableStateOf<uniffi.kshana_ffi.ChartRecord?>(null) }
    var brief by remember { mutableStateOf(BriefingCache.record) }
    var error by remember { mutableStateOf<String?>(null) }
    var tick by remember { mutableStateOf(Instant.now()) }
    // A soft tap when the hour turns while the Loop is open.
    var lastShift by remember { mutableStateOf<Instant?>(null) }
    // Time travel: an offset from now, in seconds. Zero means live.
    var travelSeconds by remember { mutableStateOf(0L) }
    var pickDate by remember { mutableStateOf(false) }
    val travelling = travelSeconds != 0L

    LaunchedEffect(Unit) {
        while (true) {
            tick = Instant.now()
            val next = lastShift
            if (next != null && !travelling && Instant.now().isAfter(next)) {
                lastShift = null
                runCatching {
                    val v = context.getSystemService(android.content.Context.VIBRATOR_SERVICE) as android.os.Vibrator
                    v.vibrate(android.os.VibrationEffect.createOneShot(30, 80))
                }
            }
            delay(1000)
        }
    }
    LaunchedEffect(moment) {
        val m0 = moment ?: return@LaunchedEffect
        lastShift = m0.shifts.firstOrNull()?.at?.toJava()
    }
    LaunchedEffect(profile, place, settings, travelSeconds) {
        // The reading is loaded as soon as the Loop opens, so the season line is there
        // before the person scrolls to the card.
        val hourKey = (System.currentTimeMillis() + travelSeconds * 1000L) / 3_600_000L
        val key = "${profile.birthInstant().taiSeconds}|${place.name}|${settings.ayanamsa}|${settings.houseSystem}|$hourKey"
        if (BriefingCache.key == key && BriefingCache.record != null) { brief = BriefingCache.record; return@LaunchedEffect }
        runCatching {
            val engine = EngineProvider.get(context)
            val at = Instants.fromInstant(Instant.now().plusSeconds(travelSeconds))
            val r = withContext(Dispatchers.Default) {
                engine.briefing(
                    profile.birthInstant(), profile.latitude, profile.longitude,
                    place.latitude, place.longitude,
                    settings.houseSystem, settings.ayanamsa, listOf("Love", "Career", "Money", "Health"), at,
                )
            }
            BriefingCache.key = key
            BriefingCache.record = r
            brief = r
            com.kshana.app.system.SadeSatiWidget.push(context, r)
        }
    }
    LaunchedEffect(profile, place, settings, travelSeconds) {
        // The real sky for the backdrop, once per load.
        runCatching {
            val engine = EngineProvider.get(context)
            val at = Instants.fromInstant(Instant.now().plusSeconds(travelSeconds))
            sky = withContext(Dispatchers.Default) {
                engine.chart(at, place.latitude, place.longitude, 0.0, settings.houseSystem, settings.ayanamsa, false)
            }
        }
    }
    LaunchedEffect(profile, place, settings, travelSeconds) {
        while (true) {
            val wait = try {
                val at = Instant.now().plusSeconds(travelSeconds)
                val m = loadMoment(context, profile, place, settings, Instants.fromInstant(at))
                moment = m
                error = null
                if (travelling) {
                    60_000L
                } else {
                    val next = m.shifts.firstOrNull()?.at?.toJava()
                    next?.let { Duration.between(Instant.now(), it).toMillis() + 400 } ?: 300_000L
                }
            } catch (e: Exception) {
                error = e.message ?: e.toString()
                5_000L
            }
            delay(wait.coerceIn(1_000L, 300_000L))
        }
    }
    val shownTick = tick.plusSeconds(travelSeconds)

    val m = moment
    // The screen dims a shade inside Rahu kala, Yamaganda and Gulika: colour as information.
    val inCaution = m?.windows?.any { it.caution && !shownTick.isBefore(it.start.toJava()) && shownTick.isBefore(it.end.toJava()) } == true
    Box(Modifier.fillMaxSize()) {
    LazyColumn(
        Modifier.fillMaxSize(),
        contentPadding = PaddingValues(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        item {
            Column(Modifier.padding(horizontal = 24.dp)) {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                    Column {
                        Text("K S H A N A", fontFamily = FontFamily.Serif, fontSize = 20.sp, letterSpacing = 4.sp, color = KshanaColors.Gold)
                        Text("क्षण · the moment", fontFamily = FontFamily.Serif, fontSize = 11.sp, letterSpacing = 1.sp, color = KshanaColors.Gold.copy(alpha = 0.55f))
                    }
                    Icon(Icons.Filled.Settings, contentDescription = "settings", tint = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onSettings).padding(8.dp))
                }
                val local = ZonedDateTime.ofInstant(shownTick, java.time.ZoneId.systemDefault())
                Text(
                    "${greeting(local.hour)}${profile.name.takeIf { it.isNotBlank() }?.let { ", ${it.substringBefore(' ')}" } ?: ""}",
                    style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text,
                )
                Row(verticalAlignment = Alignment.Bottom) {
                    Text(local.format(CLOCK), fontFamily = FontFamily.Monospace, fontSize = 30.sp, color = KshanaColors.Text)
                    Text("  ${local.format(LONG_DATE)} · ${place.name}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(bottom = 6.dp))
                }
                // The season in one quiet line: the running periods and Saturn, stated, not announced.
                brief?.let { b ->
                    Text(
                        seasonLine(b),
                        style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
                        modifier = Modifier.padding(top = 6.dp),
                    )
                }
                ErrorLine(error)
                // Time travel: step the whole loop forward or back.
                Row(Modifier.fillMaxWidth().padding(top = 8.dp), horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text(if (travelling) tr("TRAVELLING") else tr("TRAVEL"), style = MaterialTheme.typography.labelSmall, color = if (travelling) KshanaColors.Ember else KshanaColors.Muted, modifier = Modifier.width(72.dp))
                    for ((label, secs) in listOf("−1d" to -86_400L, "+1h" to 3_600L, "+1d" to 86_400L, "+7d" to 604_800L, "+30d" to 2_592_000L)) {
                        Text(
                            label, fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky,
                            modifier = Modifier.glass(10.dp).clickable { travelSeconds += secs; moment = null }.padding(horizontal = 10.dp, vertical = 6.dp),
                        )
                    }
                    Text(
                        "DATE", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Gold,
                        modifier = Modifier.glass(10.dp).border(1.dp, KshanaColors.Gold.copy(alpha = 0.6f), RoundedCornerShape(10.dp)).clickable { pickDate = true }.padding(horizontal = 10.dp, vertical = 6.dp),
                    )
                    if (travelling) {
                        Text(
                            "NOW", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Night,
                            modifier = Modifier.clip(RoundedCornerShape(10.dp)).background(KshanaColors.Gold).clickable { travelSeconds = 0L; moment = null }.padding(horizontal = 10.dp, vertical = 6.dp),
                        )
                    }
                }
                if (pickDate) {
                    val state = rememberDatePickerState(initialSelectedDateMillis = shownTick.toEpochMilli())
                    DatePickerDialog(
                        onDismissRequest = { pickDate = false },
                        confirmButton = {
                            TextButton(onClick = {
                                state.selectedDateMillis?.let { ms ->
                                    // Travel to noon of the chosen day, local time, keeping the clock live from there.
                                    val day = Instant.ofEpochMilli(ms).atZone(java.time.ZoneOffset.UTC).toLocalDate()
                                    val target = day.atTime(12, 0).atZone(java.time.ZoneId.systemDefault()).toInstant()
                                    travelSeconds = Duration.between(Instant.now(), target).seconds
                                    moment = null
                                }
                                pickDate = false
                            }) { Text("GO", color = KshanaColors.Gold) }
                        },
                        dismissButton = { TextButton(onClick = { pickDate = false }) { Text("CANCEL", color = KshanaColors.Muted) } },
                    ) { DatePicker(state = state, title = null, headline = null, showModeToggle = false) }
                }
            }
        }

        item {
            if (m == null) {
                Working("reading the sky for this second…", modifier = Modifier.padding(horizontal = 24.dp))
            } else {
                HeroCard(m, shownTick, travelling, sky, onLife)
            }
        }

        if (m != null) {
            item {
                Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    SectionLabel(tr("Your nine lives · right now"))
                    LivesGrid(m.lives, onLife)
                }
            }
            item {
                WallStrip(profile, place, settings, shownTick, onWall)
            }
            item {
                Column(Modifier.padding(horizontal = 24.dp)) {
                    PeopleToday(profile, place, settings, shownTick, onTogether)
                }
            }
            item {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    SectionLabel(tr("The hours of today"), Modifier.padding(horizontal = 24.dp))
                    HoursStrip(m.hours, shownTick, m.lives)
                    m.hora?.let { HourLoreCard(it, shownTick) }
                }
            }
            item {
                Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    var clockOpen by remember { mutableStateOf(false) }
                    Row(Modifier.fillMaxWidth().clickable { clockOpen = !clockOpen }, verticalAlignment = Alignment.CenterVertically) {
                        SectionLabel(tr("Watch the clock"), Modifier.weight(1f))
                        Text(if (clockOpen) "−" else "+", fontSize = 20.sp, color = KshanaColors.Gold)
                    }
                    val caution = m.windows.filter { it.caution && it.end.toJava().isAfter(shownTick) }.minByOrNull { it.start.seconds() }
                    if (!clockOpen && caution != null) WindowRow(caution, shownTick)
                    if (clockOpen) {
                        for (w in m.windows) WindowRow(w, shownTick)
                        Spacer(Modifier.height(4.dp))
                        for (s in m.shifts.take(10)) ShiftRow(s, shownTick)
                    }
                }
            }
            item {
                Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    var readOpen by remember { mutableStateOf(false) }
                    Text(if (readOpen) "YOUR READING −" else "YOUR READING IN FULL +", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable { readOpen = !readOpen }.padding(vertical = 6.dp))
                    if (readOpen) {
                        Text(m.taraWords.replaceFirstChar { it.uppercase() }.trimEnd('.') + ".", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                        BriefingCard(profile, place, settings, travelSeconds, onLife, onRemedy, onLoaded = { brief = it })
                    }
                    Spacer(Modifier.height(6.dp))
                    Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        GoldButton(tr("ASK"), onClick = onAsk, modifier = Modifier.weight(1f))
                        QuietButton("WHAT TO DO", onClick = onRemedy, modifier = Modifier.weight(1f))
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        QuietButton("THE WEEK, MONTH, YEAR", onClick = onAhead, modifier = Modifier.weight(1f))
                        QuietButton("DID IT HAPPEN?", onClick = onRecord, modifier = Modifier.weight(1f))
                    }
                    Text(tr("Everything is worked out on this phone from your birth time. Nothing leaves it."), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                }
            }
        }
    }
    if (inCaution) {
        Box(Modifier.fillMaxSize().background(KshanaColors.Night.copy(alpha = 0.22f)))
    }
    }
}

/** The first four things on the wall, as a strip; the whole wall is one tap away. */
@Composable
private fun WallStrip(profile: BirthProfile, place: Place, settings: EngineSettings, tick: Instant, onWall: () -> Unit) {
    val context = LocalContext.current
    val key = "${profile.birthInstant().taiSeconds}|${place.name}|${settings.ayanamsa}|${java.time.LocalDate.now()}"
    var items by remember { mutableStateOf(if (WallCache.key == key) WallCache.countdowns else null) }
    var fest by remember { mutableStateOf(if (WallCache.key == key) WallCache.festivals else null) }
    LaunchedEffect(key) {
        if (items != null) return@LaunchedEffect
        runCatching {
            val engine = EngineProvider.get(context)
            val (c, f) = withContext(Dispatchers.Default) {
                Pair(
                    engine.countdowns(profile.birthInstant(), profile.latitude, profile.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, Instants.now()),
                    engine.festivals(profile.birthInstant(), profile.latitude, profile.longitude, settings.houseSystem, settings.ayanamsa, Instants.now(), 35u),
                )
            }
            WallCache.key = key
            WallCache.countdowns = c
            WallCache.festivals = f
            items = c
            fest = f
        }
    }
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(Modifier.fillMaxWidth().padding(horizontal = 24.dp), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            SectionLabel("What you are waiting for")
            Text("THE WALL ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onWall).padding(top = 18.dp, bottom = 6.dp))
        }
        val list = items
        if (list == null) {
            Text("solving the sky ahead…", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(horizontal = 24.dp))
        } else {
            LazyRow(contentPadding = PaddingValues(horizontal = 24.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                items(list.take(6)) { c ->
                    val at = Instants.toLocal(c.at).toInstant()
                    Column(
                        Modifier.width(150.dp).glass(12.dp).clickable(onClick = onWall).padding(12.dp),
                        verticalArrangement = Arrangement.spacedBy(2.dp),
                    ) {
                        Text(c.title.uppercase(), style = MaterialTheme.typography.labelSmall, color = if (c.good) KshanaColors.Leaf else KshanaColors.Ember, maxLines = 2)
                        Text(left(tick, at), fontFamily = FontFamily.Monospace, fontSize = 18.sp, color = KshanaColors.Text)
                        Text(localText(c.at, DATE_ONLY), fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = KshanaColors.Sky)
                    }
                }
            }
        }
        val f = fest?.filter { Instants.toLocal(it.at).toInstant().isBefore(tick.plusSeconds(7 * 86_400)) && Instants.toLocal(it.at).toInstant().isAfter(tick.minusSeconds(86_400)) }
        if (!f.isNullOrEmpty()) {
            Text(
                "This week: " + f.joinToString(" · ") { "${it.name} ${localText(it.at, java.time.format.DateTimeFormatter.ofPattern("EEE"))}" },
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.TextSoft, modifier = Modifier.padding(horizontal = 24.dp),
            )
        }
    }
}

internal object PeopleCache {
    val moments = HashMap<String, MomentRecord>()
    var talkKey: String? = null
    var talk: uniffi.kshana_ffi.TalkWindowRecord? = null
}

/** The partner's day on your Loop, and the talk window for the two of you. */
@Composable
private fun PeopleToday(profile: BirthProfile, place: Place, settings: EngineSettings, tick: Instant, onTogether: () -> Unit) {
    val context = LocalContext.current
    val people = remember { ProfileStore.partners(context) }
    if (people.isEmpty()) return
    val hourKey = System.currentTimeMillis() / 3_600_000L
    var moments by remember { mutableStateOf<Map<String, MomentRecord>>(emptyMap()) }
    var talk by remember { mutableStateOf(PeopleCache.talk) }
    LaunchedEffect(hourKey, people) {
        runCatching {
            val engine = EngineProvider.get(context)
            val out = HashMap<String, MomentRecord>()
            for (p in people.take(4)) {
                val k = "${p.name}|$hourKey|${place.name}"
                out[p.name] = PeopleCache.moments[k] ?: withContext(Dispatchers.Default) {
                    engine.forecastMoment(p.birthInstant(), p.latitude, p.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, Instants.now())
                }.also { PeopleCache.moments[k] = it }
            }
            moments = out
            val first = people[ProfileStore.selectedPartner(context).coerceIn(0, people.lastIndex)]
            val tk = "${first.name}|${java.time.LocalDate.now()}|${place.name}"
            if (PeopleCache.talkKey != tk) {
                val w = withContext(Dispatchers.Default) {
                    engine.talkWindow(profile.birthInstant(), profile.latitude, profile.longitude, first.birthInstant(), first.latitude, first.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, Instants.now(), 7u)
                }
                PeopleCache.talkKey = tk
                PeopleCache.talk = w
                talk = w
            }
        }
    }
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            SectionLabel("Your people today")
            Text("US ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onTogether).padding(top = 18.dp, bottom = 6.dp))
        }
        for (p in people.take(4)) {
            val pm = moments[p.name]
            val rel = ProfileStore.relationOf(context, p.name)
            Row(
                Modifier.fillMaxWidth().glass(12.dp).clickable(onClick = onTogether).padding(12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(Modifier.weight(1f)) {
                    Text("${p.name.substringBefore(' ')} · $rel", style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text)
                    if (pm == null) {
                        Text("reading…", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                    } else {
                        val top = pm.lives.maxByOrNull { it.score }
                        val low = pm.lives.minByOrNull { it.score }
                        Text("Going well for them: ${top?.life?.let { lifeName(it).lowercase() }}. Go gently today with: ${low?.life?.let { lifeName(it).lowercase() }}.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.TextSoft)
                        if (pm.saturnActive) Text(firstSentence(pm.saturnWords.replaceFirstChar { it.uppercase() }), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Ember)
                    }
                }
                if (pm != null) ScoreRing(pm.overall, size = 48.dp, stroke = 4.dp)
            }
        }
        val w = talk
        if (w != null) {
            val s = Instants.toLocal(w.start).toInstant()
            Row(Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).background(KshanaColors.Gold.copy(alpha = 0.10f)).border(1.dp, KshanaColors.Gold.copy(alpha = 0.5f), RoundedCornerShape(12.dp)).padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text("THE TALK WINDOW", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                    Text("${localText(w.start, WEEKDAY_DAY)} · ${hm(w.start)} – ${hm(w.end)}", fontFamily = FontFamily.Monospace, fontSize = 15.sp, color = KshanaColors.Text)
                    Text("The best hour this week for the two of you to talk: both of you will be at your warmest and most patient.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.TextSoft)
                }
                Text(if (s.isAfter(tick)) "in ${countdown(tick, s)}" else "now", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Gold)
            }
        }
    }
}

/** One briefing per profile, place and hour is kept for the process so the Loop stays instant. */
internal object BriefingCache {
    var key: String? = null
    var record: BriefingRecord? = null
}

/**
 * The reading, as an astrologer would give it: the birth Moon (so it can be checked),
 * the running periods with their ends, Saturn's phase and its exit date, the moment in
 * words, the days to circle this month and the remedy in brief. Loads once per hour.
 */
@Composable
private fun BriefingCard(
    profile: BirthProfile,
    place: Place,
    settings: EngineSettings,
    travelSeconds: Long,
    onLife: (String) -> Unit,
    onRemedy: () -> Unit,
    onLoaded: (BriefingRecord) -> Unit = {},
) {
    val context = LocalContext.current
    val hourKey = (System.currentTimeMillis() + travelSeconds * 1000L) / 3_600_000L
    val key = "${profile.birthInstant().taiSeconds}|${place.name}|${settings.ayanamsa}|${settings.houseSystem}|$hourKey"
    var b by remember { mutableStateOf(if (BriefingCache.key == key) BriefingCache.record else null) }
    var error by remember { mutableStateOf<String?>(null) }
    var open by remember { mutableStateOf(false) }
    LaunchedEffect(key) {
        if (BriefingCache.key == key && BriefingCache.record != null) { b = BriefingCache.record; BriefingCache.record?.let(onLoaded); return@LaunchedEffect }
        try {
            val engine = EngineProvider.get(context)
            val at = Instants.fromInstant(Instant.now().plusSeconds(travelSeconds))
            val r = withContext(Dispatchers.Default) {
                engine.briefing(
                    profile.birthInstant(), profile.latitude, profile.longitude,
                    place.latitude, place.longitude,
                    settings.houseSystem, settings.ayanamsa, listOf("Love", "Career", "Money", "Health"), at,
                )
            }
            BriefingCache.key = key
            BriefingCache.record = r
            b = r
            error = null
            onLoaded(r)
            com.kshana.app.system.SadeSatiWidget.push(context, r)
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }
    Column(
        Modifier.fillMaxWidth().glass(18.dp).padding(18.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("YOUR READING", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        ErrorLine(error)
        val r = b
        if (r == null) {
            if (error == null) Working("writing your reading…")
            return@Column
        }
        val shown = if (open) r.paragraphs else r.paragraphs.take(1)
        for (p in shown) Text(p, fontFamily = FontFamily.Serif, fontSize = 17.sp, lineHeight = 25.sp, color = KshanaColors.Text)
        Text(
            if (open) "LESS −" else "THE WHOLE READING +",
            style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
            modifier = Modifier.clickable { open = !open }.padding(vertical = 2.dp),
        )
        if (open) {
            HorizontalDivider(color = KshanaColors.LineSoft)
            Text("YOUR BEST DAYS · THE NEXT THIRTY", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            for (c in r.circled) {
                Row(Modifier.fillMaxWidth().clickable { onLife(c.life) }.padding(vertical = 3.dp), verticalAlignment = Alignment.Top) {
                    Text(lifeGlyph(c.life), fontSize = 14.sp, color = scoreColor(c.score), modifier = Modifier.width(22.dp).padding(top = 2.dp))
                    Column(Modifier.weight(1f)) {
                        Text("${c.life} · ${localText(c.at, WEEKDAY_DAY)}", style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text)
                        Text(c.words.substringAfter(". ", c.words), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                    }
                    Text(c.score.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 16.sp, color = scoreColor(c.score))
                }
            }
            r.remedy?.let { rem ->
                HorizontalDivider(color = KshanaColors.LineSoft)
                Text("WHAT TO DO ABOUT IT", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text(rem.cause, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                for (t in rem.top) {
                    Row(Modifier.padding(vertical = 2.dp), verticalAlignment = Alignment.Top) {
                        Text(kindWord(t.kind), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.width(78.dp).padding(top = 3.dp))
                        Column(Modifier.weight(1f)) {
                            Text(t.title, style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text)
                            Text(t.why, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
                        }
                    }
                }
                QuietButton("ALL ${rem.total} REMEDIES", onClick = onRemedy, modifier = Modifier.fillMaxWidth())
            }
        }
    }
}

/** The running hour's lord with its deity, colour, direction, metal and mantra. */
@Composable
private fun HourLoreCard(h: uniffi.kshana_ffi.HoraRecord, tick: Instant) {
    var open by remember { mutableStateOf(false) }
    Column(
        Modifier.padding(horizontal = 24.dp).fillMaxWidth().glass(14.dp).clickable { open = !open }.padding(14.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("This hour: ${hourShort(h.lord).lowercase()}", fontFamily = FontFamily.Serif, fontSize = 18.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
            Text("${hm(h.start)} – ${hm(h.end)} · ${countdown(tick, h.end.toJava())} left", fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = KshanaColors.Sky)
        }
        Text(hourNature(h.lord).replaceFirstChar { it.uppercase() } + ".", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        Text("To pray to: ${h.deity} · wear ${h.colour.lowercase()} · face ${h.direction.lowercase()}", style = MaterialTheme.typography.labelMedium, color = KshanaColors.Gold)
        if (open) {
            Text(h.mantra, fontFamily = FontFamily.Serif, fontSize = 17.sp, color = KshanaColors.Text)
            Text("Good for ${h.forWords}. Keep away from ${h.avoid}.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            val context = LocalContext.current
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.CenterVertically) {
                if (com.kshana.app.system.Chant.running) {
                    QuietButton("STOP · ${com.kshana.app.system.Chant.count} OF 108", onClick = { com.kshana.app.system.Chant.stop() }, modifier = Modifier.weight(1f))
                } else {
                    QuietButton("CHANT IT ×108 · SPOKEN", onClick = { com.kshana.app.system.Chant.start(context, h.mantra) }, modifier = Modifier.weight(1f))
                    QuietButton("×11", onClick = { com.kshana.app.system.Chant.start(context, h.mantra, 11) })
                }
            }
            Text("Spoken slowly by the phone's own voice; no recording, no network.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
        Text(if (open) "LESS −" else "THE MANTRA +", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
    }
}

/**
 * The season in plain words: what the present stretch of life is about and until when,
 * and the long testing time if one is on. The period's key is parsed, never shown.
 */
private fun seasonLine(b: BriefingRecord): String {
    // The engine's key: "Moon mahadasha (to 10 Oct 2030) › Mercury antardasha (to 9 Jan 2028)".
    val antar = b.dasha.split(" › ").getOrNull(1)
    val antarLord = antar?.substringBefore(" ")?.trim()
    val antarEnd = antar?.substringAfter("(to ")?.substringBefore(")")?.trim()
    val first = if (!antarEnd.isNullOrBlank() && antarLord != null) "Until $antarEnd, life moves through ${chapterTheme(antarLord)}." else ""
    val saturn = if (b.saturnActive) " " + firstSentence(b.saturnWords.replaceFirstChar { it.uppercase() }) else ""
    return (first + saturn).trim()
}

private fun greeting(hour: Int): String = when (hour) {
    in 4..11 -> tr("Good morning")
    in 12..16 -> tr("Good afternoon")
    in 17..20 -> tr("Good evening")
    else -> tr("Good night")
}

@Composable
private fun HeroCard(m: MomentRecord, tick: Instant, travelling: Boolean, sky: uniffi.kshana_ffi.ChartRecord?, onLife: (String) -> Unit) {
    val next = m.shifts.firstOrNull()
    Box(Modifier.padding(horizontal = 24.dp).fillMaxWidth().glass(18.dp)) {
    if (sky != null) SkyBackdrop(sky, m.band)
    Column(
        Modifier.fillMaxWidth().padding(18.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text(m.dayTitle.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(18.dp)) {
            RingWithTrace(m, tick, size = 96.dp, stroke = 7.dp)
            Column(Modifier.weight(1f)) {
                Text(m.title.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text(m.headline.substringBefore(" ·"), style = MaterialTheme.typography.titleLarge, color = bandColor(m.band))
                if (travelling) {
                    Spacer(Modifier.height(6.dp))
                    Text("you are looking at", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Ember)
                    Text(localText(m.at, DATE_TIME), fontFamily = FontFamily.Monospace, fontSize = 18.sp, color = KshanaColors.Gold)
                    if (next != null) Text("then: ${hm(next.at)} · ${next.what}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
                } else if (next != null) {
                    Spacer(Modifier.height(6.dp))
                    Text(tr("changes in"), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                    Text(countdown(tick, next.at.toJava()), fontFamily = FontFamily.Monospace, fontSize = 26.sp, color = KshanaColors.Gold.copy(alpha = com.kshana.app.ui.theme.breath()))
                    Text("${hm(next.at)} · ${next.what}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
                }
            }
        }
        val top = m.lives.maxByOrNull { it.score }
        val low = m.lives.minByOrNull { it.score }
        if (top != null && low != null) {
            Text(top.headline, fontFamily = FontFamily.Serif, fontSize = 19.sp, lineHeight = 26.sp, color = KshanaColors.Text)
            Text("${lifeGlyph(top.life)} ${top.life} is your strongest now · go gently with ${low.life.lowercase()}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
        // The next good minute: for the weakest of the four main lives, the next hour that serves it.
        val mains = listOf("Love", "Career", "Money", "Health")
        val focus = m.lives.filter { it.life in mains }.minByOrNull { it.score }?.life ?: "Love"
        val fi = LIFE_ORDER.indexOf(focus).coerceAtLeast(0)
        val g = m.nextGood.getOrNull(fi)
        Row(
            Modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).background(KshanaColors.Night.copy(alpha = 0.5f)).clickable { onLife(focus) }.padding(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text("YOUR NEXT GOOD MINUTE FOR ${focus.uppercase()}", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                if (g == null) {
                    Text("None left today; tomorrow's sunrise starts a new set.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                } else {
                    Text(if (g.now) "Now, until ${hm(g.end)} · ${hourShort(g.lord).lowercase()}" else "${hm(g.start)} – ${hm(g.end)} · ${hourShort(g.lord).lowercase()}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
                }
            }
            if (g != null) Text(if (g.now) "now" else countdown(tick, g.start.toJava()), fontFamily = FontFamily.Monospace, fontSize = 20.sp, color = KshanaColors.Gold.copy(alpha = com.kshana.app.ui.theme.breath()))
        }
    }
    }
}

/** The real sky behind the words: the twelve signs as a faint band with the planets at their true places, the ascendant to the left. */
@Composable
private fun SkyBackdrop(chart: uniffi.kshana_ffi.ChartRecord, band: String) {
    val asc = chart.cuspsTropical.firstOrNull() ?: 0.0
    val accent = bandColor(band)
    Canvas(Modifier.fillMaxWidth().height(230.dp)) {
        val cx = size.width * 0.78f
        val cy = size.height * 0.55f
        val r = size.height * 0.62f
        // the band of the ecliptic
        drawCircle(KshanaColors.Line.copy(alpha = 0.35f), r, Offset(cx, cy), style = Stroke(1f))
        drawCircle(KshanaColors.Line.copy(alpha = 0.25f), r * 0.78f, Offset(cx, cy), style = Stroke(1f))
        for (k in 0 until 12) {
            val a = Math.toRadians(180.0 + (k * 30.0 - asc))
            val x1 = cx + (r * 0.78f) * Math.cos(a).toFloat()
            val y1 = cy - (r * 0.78f) * Math.sin(a).toFloat()
            val x2 = cx + r * Math.cos(a).toFloat()
            val y2 = cy - r * Math.sin(a).toFloat()
            drawLine(KshanaColors.Line.copy(alpha = 0.3f), Offset(x1, y1), Offset(x2, y2), 1f)
        }
        // the planets
        for (b in chart.bodies) {
            val a = Math.toRadians(180.0 + (b.tropical - asc))
            val rr = r * 0.89f
            val x = cx + rr * Math.cos(a).toFloat()
            val y = cy - rr * Math.sin(a).toFloat()
            val c = when (b.name) {
                "Sun" -> KshanaColors.Gold
                "Moon" -> KshanaColors.Text
                "Mars" -> KshanaColors.Ember
                "Jupiter" -> KshanaColors.Gold
                "Venus" -> KshanaColors.Sky
                "Saturn" -> KshanaColors.Muted
                else -> KshanaColors.Sky
            }
            val size = if (b.name == "Sun" || b.name == "Moon") 4.5f else 3f
            drawCircle(c.copy(alpha = 0.55f), size, Offset(x, y))
            drawCircle(c.copy(alpha = 0.12f), size * 3, Offset(x, y))
        }
        // the horizon
        drawLine(accent.copy(alpha = 0.25f), Offset(cx - r, cy), Offset(cx + r, cy), 1f)
    }
}

/** The score ring with a thin trace of the day's hours around it: the day's shape at a glance. */
@Composable
private fun RingWithTrace(m: MomentRecord, tick: Instant, size: androidx.compose.ui.unit.Dp, stroke: androidx.compose.ui.unit.Dp) {
    Box(Modifier.size(size + 14.dp), contentAlignment = Alignment.Center) {
        Canvas(Modifier.fillMaxSize()) {
            val n = m.hours.size
            if (n > 0) {
                val inset = 1.5f
                val arc = Size(this.size.width - 3f, this.size.height - 3f)
                val sweep = 360f / n
                m.hours.forEachIndexed { i, h ->
                    val mean = if (h.fit.isEmpty()) 50.0 else h.fit.average()
                    val past = !tick.isBefore(h.end.toJava())
                    val now = !tick.isBefore(h.start.toJava()) && tick.isBefore(h.end.toJava())
                    val c = if (h.caution) KshanaColors.Ember else scoreColor(mean)
                    drawArc(c.copy(alpha = if (now) 0.95f else if (past) 0.55f else 0.22f), -90f + i * sweep, sweep - 1.5f, false, Offset(inset, inset), arc, style = Stroke(3f, cap = StrokeCap.Butt))
                }
            }
        }
        ScoreRing(m.overall, size = size, stroke = stroke)
    }
}

/** A ring filled to `score`/100 with the number inside. */
@Composable
internal fun ScoreRing(score: Double, size: androidx.compose.ui.unit.Dp, stroke: androidx.compose.ui.unit.Dp, label: String? = null) {
    val color = scoreColor(score)
    // The ring draws itself to its number, and carries a soft light of its own colour.
    val sweep by androidx.compose.animation.core.animateFloatAsState(
        targetValue = (360.0 * (score / 100.0)).toFloat(),
        animationSpec = androidx.compose.animation.core.tween(1100, easing = androidx.compose.animation.core.FastOutSlowInEasing),
        label = "ring",
    )
    Box(Modifier.size(size).halo(color, radiusFactor = 0.62f, alpha = 0.20f), contentAlignment = Alignment.Center) {
        Canvas(Modifier.fillMaxSize()) {
            val s = stroke.toPx()
            val inset = s / 2
            val arcSize = Size(this.size.width - s, this.size.height - s)
            drawArc(KshanaColors.Line.copy(alpha = 0.7f), -90f, 360f, false, Offset(inset, inset), arcSize, style = Stroke(s, cap = StrokeCap.Round))
            drawArc(color, -90f, sweep, false, Offset(inset, inset), arcSize, style = Stroke(s, cap = StrokeCap.Round))
            // a brighter head on the arc, like a planet on its orbit
            if (sweep > 2f) {
                val a = Math.toRadians((-90f + sweep).toDouble())
                val r = (this.size.width - s) / 2f
                drawCircle(KshanaColors.Text, s * 0.42f, Offset(center.x + r * Math.cos(a).toFloat(), center.y + r * Math.sin(a).toFloat()))
            }
        }
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text(score.toInt().toString(), fontFamily = FontFamily.Monospace, fontWeight = FontWeight.Medium, fontSize = (size.value / 3.2).sp, color = KshanaColors.Text)
            if (label != null) Text(label, style = MaterialTheme.typography.labelSmall, color = color)
        }
    }
}

@Composable
private fun LivesGrid(lives: List<LifeScoreRecord>, onLife: (String) -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        for (row in lives.chunked(3)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (l in row) LifeTile(l, Modifier.weight(1f)) { onLife(l.life) }
                repeat(3 - row.size) { Spacer(Modifier.weight(1f)) }
            }
        }
    }
}

@Composable
private fun LifeTile(l: LifeScoreRecord, modifier: Modifier, onClick: () -> Unit) {
    val c = bandColor(l.band)
    Column(
        modifier.glass(14.dp).clickable(onClick = onClick).padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(lifeGlyph(l.life), fontSize = 14.sp, color = c)
            Text(l.score.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 22.sp, color = KshanaColors.Text)
        }
        Text(lifeName(l.life), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
        Box(Modifier.fillMaxWidth().height(4.dp).clip(RoundedCornerShape(2.dp)).background(KshanaColors.Line)) {
            Box(Modifier.fillMaxWidth((l.score / 100.0).toFloat().coerceIn(0.02f, 1f)).height(4.dp).background(c))
        }
        Text(bandWord(l.band), style = MaterialTheme.typography.labelSmall, color = c)
    }
}

@Composable
private fun HoursStrip(hours: List<HourRecord>, tick: Instant, lives: List<LifeScoreRecord>) {
    if (hours.isEmpty()) return
    val state = rememberLazyListState()
    val currentIndex = hours.indexOfFirst { !tick.isBefore(it.start.toJava()) && tick.isBefore(it.end.toJava()) }
    LaunchedEffect(hours.firstOrNull()?.start, currentIndex) {
        if (currentIndex >= 0) state.animateScrollToItem((currentIndex - 1).coerceAtLeast(0))
    }
    LazyRow(state = state, contentPadding = PaddingValues(horizontal = 24.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        items(hours) { h ->
            val start = h.start.toJava()
            val end = h.end.toJava()
            val now = !tick.isBefore(start) && tick.isBefore(end)
            val past = !tick.isBefore(end)
            val mean = if (h.fit.isEmpty()) 50.0 else h.fit.average()
            val bestIdx = h.fit.withIndex().maxByOrNull { it.value }?.index
            val best = bestIdx?.let { lives.getOrNull(it)?.life } ?: ""
            Column(
                Modifier.width(92.dp).clip(RoundedCornerShape(12.dp))
                    .background(if (now) KshanaColors.Gold.copy(alpha = 0.14f) else KshanaColors.Surface)
                    .then(if (now) Modifier.border(1.dp, KshanaColors.Gold, RoundedCornerShape(12.dp)) else Modifier)
                    .padding(10.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(3.dp),
            ) {
                Text(hm(h.start), fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = if (past) KshanaColors.Muted.copy(alpha = 0.5f) else KshanaColors.Muted)
                Text(hourShort(h.lord), fontFamily = FontFamily.Serif, fontSize = 13.sp, lineHeight = 15.sp, textAlign = TextAlign.Center, color = if (past) KshanaColors.TextSoft.copy(alpha = 0.5f) else KshanaColors.Text)
                Box(Modifier.size(8.dp).clip(CircleShape).background(if (h.caution) KshanaColors.Ember else scoreColor(mean)))
                Text(if (h.caution) "pause" else best, style = MaterialTheme.typography.labelSmall, color = if (h.caution) KshanaColors.Ember else KshanaColors.Muted, textAlign = TextAlign.Center)
            }
        }
    }
}

@Composable
private fun WindowRow(w: WindowRecord, tick: Instant) {
    val active = !tick.isBefore(w.start.toJava()) && tick.isBefore(w.end.toJava())
    val past = !tick.isBefore(w.end.toJava())
    val alpha = if (past) 0.45f else 1f
    val c = if (w.caution) KshanaColors.Ember else KshanaColors.Leaf
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(8.dp).clip(CircleShape).background(c.copy(alpha = alpha)))
        Spacer(Modifier.width(10.dp))
        Text(w.label, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text.copy(alpha = alpha), modifier = Modifier.weight(1f))
        Text("${hm(w.start)} – ${hm(w.end)}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = if (active) c else KshanaColors.Muted.copy(alpha = alpha))
        if (active) Text("  now", style = MaterialTheme.typography.labelSmall, color = c)
        if (past) Text("  done", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted.copy(alpha = alpha))
    }
    if (!past) {
        Text(
            if (w.caution) "Finish things, start nothing new." else "The best minutes of the day to begin.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(start = 18.dp),
        )
    }
}

@Composable
private fun ShiftRow(s: ShiftRecord, tick: Instant) {
    val context = LocalContext.current
    val c: Color = when (s.kind) {
        "kala" -> KshanaColors.Ember
        "star", "sign", "tithi" -> KshanaColors.Sky
        "dasha" -> KshanaColors.Gold
        else -> KshanaColors.Muted
    }
    val at = s.at.toJava().toEpochMilli()
    var set by remember(at) { mutableStateOf(com.kshana.app.system.Alerts.has(context, at)) }
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(hm(s.at), fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Muted, modifier = Modifier.width(52.dp))
        Text(s.what, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft, modifier = Modifier.weight(1f))
        Text("in ${countdown(tick, s.at.toJava())}", fontFamily = FontFamily.Monospace, fontSize = 10.sp, color = c)
        Text(
            if (set) "🔔" else "🔕", fontSize = 13.sp,
            modifier = Modifier.clickable { set = com.kshana.app.system.Alerts.toggle(context, at, s.what, "${hm(s.at)} · ${s.what}") }.padding(start = 10.dp, top = 4.dp, bottom = 4.dp),
            color = if (set) KshanaColors.Gold else KshanaColors.Muted.copy(alpha = 0.5f),
        )
    }
}
