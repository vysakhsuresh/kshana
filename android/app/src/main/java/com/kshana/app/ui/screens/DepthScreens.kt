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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.data.PlaceClock
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.time.format.DateTimeFormatter

/** A plain page frame: label, title, a line of help, then the body. */
@Composable
private fun Page(label: String, title: String, help: String, onBack: () -> Unit, body: @Composable () -> Unit) {
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("‹ BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onBack).padding(vertical = 4.dp))
        Text(label, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text(title, style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(help, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        body()
    }
}

/**
 * Your gifts: the patterns in the birth chart that the classic texts name, said plainly.
 * The gifts come first; the patterns that ask for care follow, each with what helps.
 */
@Composable
fun GiftsScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit, onRemedy: () -> Unit) {
    val context = LocalContext.current
    var list by remember { mutableStateOf<List<uniffi.kshana_ffi.ChartYogaRecord>?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    val expert = com.kshana.app.data.Expert.on(context)
    LaunchedEffect(profile) {
        runCatching {
            val engine = EngineProvider.get(context)
            list = withContext(Dispatchers.Default) { engine.chartYogas(profile.birthInstant(), profile.latitude, profile.longitude, settings.ayanamsa, Instants.now()) }
        }.onFailure { error = it.message }
    }
    Page("YOUR GIFTS", "What you were born with", "The strengths and the tender spots your birth shows, and when each one comes forward in life.", onBack) {
        ErrorLine(error)
        val l = list ?: run { if (error == null) Working("reading the patterns in your birth…"); return@Page }
        // One card per gift: the same pattern formed by several pairs of planets is one
        // gift with several times of life, strongest first.
        val merged = l.groupBy { it.good to it.title }.values.map { same ->
            val top = same.maxBy { it.strength }
            top.copy(`when` = same.map { it.`when` }.filter { it.isNotBlank() }.distinct().joinToString(" "))
        }
        val gifts = merged.filter { it.good }.sortedByDescending { it.strength }
        val tend = merged.filter { !it.good }.sortedByDescending { it.strength }
        if (gifts.isNotEmpty()) SectionLabel("Your gifts")
        for (y in gifts) YogaCard(y, expert, null)
        if (tend.isNotEmpty()) SectionLabel("What asks for care")
        for (y in tend) YogaCard(y, expert, onRemedy)
        if (l.isEmpty()) Text("No strong pattern stands out; your life is shaped more by its seasons than by any one gift.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
    }
}

@Composable
private fun YogaCard(y: uniffi.kshana_ffi.ChartYogaRecord, expert: Boolean, onRemedy: (() -> Unit)?) {
    Column(Modifier.fillMaxWidth().glass(16.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Row {
            Text(y.title, fontFamily = FontFamily.Serif, fontSize = 18.sp, lineHeight = 24.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
            Text("●".repeat((y.strength * 5).toInt().coerceIn(1, 5)), fontSize = 10.sp, color = if (y.good) KshanaColors.Gold else KshanaColors.Ember)
        }
        Text(y.meaning, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
        if (y.`when`.isNotBlank()) Text(y.`when`, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
        if (y.care.isNotBlank()) Text(y.care, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Leaf)
        if (onRemedy != null) Text("WHAT HELPS ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onRemedy).padding(vertical = 4.dp))
        if (expert) Text("${y.yoga} · ${y.source}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    }
}

/**
 * The calendar where you are: today's date in the Malayalam, Tamil and other solar
 * calendars, the lunar month and fortnight, and the festivals of the coming weeks.
 */
@Composable
fun AlmanacScreen(place: Place, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    var days by remember { mutableStateOf<List<uniffi.kshana_ffi.CalendarDayRecord>?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    val expert = com.kshana.app.data.Expert.on(context)
    LaunchedEffect(place) {
        runCatching {
            val engine = EngineProvider.get(context)
            val today = java.time.LocalDate.now(com.kshana.app.data.CityClocks.of(place.zoneId))
            days = withContext(Dispatchers.Default) {
                engine.calendarDays(place.latitude, place.longitude, PlaceClock.offsetMinutes(place), today.year.toLong(), today.monthValue.toUInt(), today.dayOfMonth.toUInt(), 45u, settings.ayanamsa)
            }
        }.onFailure { error = it.message }
    }
    Page("CALENDAR", "The days ahead", "Worked out for ${place.name} from the real sky: the dates, the moon's month and the festivals.", onBack) {
        ErrorLine(error)
        val d = days ?: run { if (error == null) Working("working out the calendar…"); return@Page }
        val today = d.firstOrNull() ?: return@Page
        Column(Modifier.fillMaxWidth().glass(18.dp).padding(18.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Text("TODAY · ${today.weekday.uppercase()} ${today.day} ${java.time.Month.of(today.month.toInt()).name.take(3)} ${today.year}", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("${today.lunarMonth}${if (today.extraMonth) " (the extra month)" else ""} · ${today.fortnight}", fontFamily = FontFamily.Serif, fontSize = 20.sp, color = KshanaColors.Text)
            for (s in today.solar) Text("${s.calendar.replaceFirstChar { it.uppercase() }}: ${s.day} ${s.month} ${s.year}", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
            Text("Indian national date: ${today.nationalDate} · ${today.season.replaceFirstChar { it.uppercase() }}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            if (today.festivals.isNotEmpty()) Text(today.festivals.joinToString(" · "), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Gold)
            if (expert) {
                Text("Tithi ${today.tithi}${if (today.tithiRepeated) " (vriddhi)" else ""}${today.tithiSkipped?.let { " · kshaya: $it" } ?: ""} · Nakshatra ${today.nakshatra}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
                Text("Samvatsara ${today.samvatsara} (N: ${today.samvatsaraNorth}) · Shaka ${today.shaka} · Vikram ${today.vikram} · Kali ${today.kali} · ${today.ritu} · ${today.ayana}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            }
        }
        SectionLabel("Festivals and holy days")
        val fmt = DateTimeFormatter.ofPattern("EEE d MMM uuuu")
        val withFest = d.filter { it.festivals.isNotEmpty() }
        for (x in withFest) {
            Row(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                Text(java.time.LocalDate.of(x.year.toInt(), x.month.toInt(), x.day.toInt()).format(fmt), fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky, modifier = Modifier.padding(end = 12.dp))
                Text(x.festivals.joinToString(" · "), style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Text)
            }
        }
        if (withFest.isEmpty()) Text("No festival falls in the next six weeks here.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
    }
}

/** Expert: the complete KP table for the birth chart. */
@Composable
fun KpFullScreen(profile: BirthProfile, place: Place, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    var kp by remember { mutableStateOf<uniffi.kshana_ffi.KpFullRecord?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(profile) {
        runCatching {
            val engine = EngineProvider.get(context)
            kp = withContext(Dispatchers.Default) { engine.kpFull(profile.birthInstant(), profile.latitude, profile.longitude, settings.ayanamsa, Instants.now(), place.latitude, place.longitude) }
        }.onFailure { error = it.message }
    }
    Page("CHART PAGES · KP", "KP, in full", "Cusps and planets with star, sub, sub-sub and sub-sub-sub lords; house significators in four levels; cuspal interlinks and ruling planets now.", onBack) {
        ErrorLine(error)
        val k = kp ?: run { if (error == null) Working("casting the KP table…"); return@Page }
        val mono: @Composable (String, androidx.compose.ui.graphics.Color) -> Unit = { s, c -> Text(s, fontFamily = FontFamily.Monospace, fontSize = 11.sp, lineHeight = 15.sp, color = c) }
        fun dms(d: Double): String {
            val sign = listOf("Ar", "Ta", "Ge", "Cn", "Le", "Vi", "Li", "Sc", "Sg", "Cp", "Aq", "Pi")[((d / 30).toInt()).mod(12)]
            val x = d.mod(30.0); val dd = x.toInt(); val m = ((x - dd) * 60).toInt(); val s = (((x - dd) * 60 - m) * 60).toInt()
            return "%s %02d°%02d′%02d″".format(sign, dd, m, s)
        }
        fun ab(p: String) = p.take(2)
        mono("Ayanamsa %.6f°".format(k.ayanamsaDeg), KshanaColors.Muted)
        SectionLabel("Cusps")
        mono("CUSP  POSITION        SGN STR SUB SS  SSS  SIGNIFIES", KshanaColors.Gold)
        for (c in k.cusps) mono("%-5s %-15s %-3s %-3s %-3s %-3s %-4s %s".format(c.point.take(5), dms(c.siderealDeg), ab(c.signLord), ab(c.starLord), ab(c.subLord), ab(c.subSubLord), ab(c.subSubSubLord), c.signifies.joinToString(",")), KshanaColors.Text)
        SectionLabel("Planets")
        mono("PLAN  POSITION        H  SGN STR SUB SS  SSS  SIGNIFIES", KshanaColors.Gold)
        for (p in k.planets) mono("%-5s %-15s %-2d %-3s %-3s %-3s %-3s %-4s %s".format(p.point.take(4) + if (p.retrograde) "R" else "", dms(p.siderealDeg), p.house.toInt(), ab(p.signLord), ab(p.starLord), ab(p.subLord), ab(p.subSubLord), ab(p.subSubSubLord), p.signifies.joinToString(",")), KshanaColors.Text)
        SectionLabel("Cuspal interlinks · four steps")
        for (c in k.cusps) if (c.steps.isNotEmpty()) mono("${c.point}: " + c.steps.joinToString("  →  ") { "${ab(it.planet)}[${it.houses.joinToString(",")}]" }, KshanaColors.TextSoft)
        SectionLabel("House significators")
        mono("H   L1 STAR-OF-OCC  L2 OCC  L3 STAR-OF-OWN  L4 OWN   ORDER", KshanaColors.Gold)
        for (h in k.houses) mono("%-3d %-15s %-7s %-15s %-6s %s".format(h.house.toInt(), h.starOfOccupants.joinToString(",") { ab(it) }, h.occupants.joinToString(",") { ab(it) }, h.starOfOwner.joinToString(",") { ab(it) }, ab(h.owner), h.ordered.joinToString(",") { ab(it) }) + if (h.ruling.isNotEmpty()) "  RP:" + h.ruling.joinToString(",") { ab(it) } else "", KshanaColors.Text)
        SectionLabel("Rahu and Ketu as agents")
        for (n in k.nodes) mono("${n.node}: conj ${n.conjoined.joinToString(",").ifEmpty { "—" }} · asp ${n.aspecting.joinToString(",").ifEmpty { "—" }} · sign ${n.signLord} · star ${n.starLord}", KshanaColors.TextSoft)
        SectionLabel("Ruling planets now")
        mono("Day ${k.ruling.dayLord} · Moon ${k.ruling.moon.joinToString("/")} · Lagna ${k.ruling.lagna.joinToString("/")}", KshanaColors.Text)
        mono("Order: ${k.ruling.ordered.joinToString(", ")}" + if (k.ruling.inRetroStar.isNotEmpty()) " · in retro star: ${k.ruling.inRetroStar.joinToString(",")}" else "", KshanaColors.Gold)
        for (n in k.notes) Text(n, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
    }
}

/** Expert: every dasha system the engine knows, the running periods first. */
@Composable
fun DashasMoreScreen(profile: BirthProfile, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    var systems by remember { mutableStateOf<List<uniffi.kshana_ffi.DashaSystemRecord>?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(profile) {
        runCatching {
            val engine = EngineProvider.get(context)
            systems = withContext(Dispatchers.Default) { engine.dashaSystemsMoreAt(profile.birthInstant(), profile.latitude, profile.longitude, settings.ayanamsa, Instants.now()) }
        }.onFailure { error = it.message }
    }
    val fmt = DateTimeFormatter.ofPattern("d MMM uuuu")
    Page("CHART PAGES · DASHAS", "Every period system", "Narayana, Sthira, Shoola, Drig and the conditional nakshatra dashas, with whether each applies to this chart by BPHS ch. 46.", onBack) {
        ErrorLine(error)
        val l = systems ?: run { if (error == null) Working("running every period system…"); return@Page }
        for (s in l) {
            Column(Modifier.fillMaxWidth().glass(14.dp).padding(14.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text("${s.system.uppercase()}${if (s.applies) " · APPLIES" else " · conditional, not this chart"}", style = MaterialTheme.typography.labelSmall, color = if (s.applies) KshanaColors.Gold else KshanaColors.Muted)
                for (p in s.running) Text("${"  ".repeat(p.level.toInt())}${p.label}  ${Instants.toLocal(p.start).format(fmt)} – ${Instants.toLocal(p.end).format(fmt)}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Text)
            }
        }
    }
}
