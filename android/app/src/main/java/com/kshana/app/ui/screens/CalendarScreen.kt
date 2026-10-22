package com.kshana.app.ui.screens

import android.Manifest
import android.content.ContentUris
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.provider.CalendarContract
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.data.ProfileStore
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.sky.KshanaFonts
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass
import java.time.DateTimeException
import java.time.Instant
import java.time.YearMonth
import java.time.ZoneId
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter
import java.time.temporal.ChronoUnit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.EclipseRecord
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.ObservanceRecord

/** An appointment from the phone's calendar, read and never stored. */
internal data class CalEvent(val id: Long, val title: String, val begin: Long, val end: Long)

/** One appointment with how its hour stands for the person. */
internal data class CalCheck(
    val event: CalEvent,
    val score: Double,
    val window: String?,
    val dayWords: String,
    val better: Pair<Long, Long>?,
    val betterScore: Double,
)

internal object CalendarCheck {
    private const val K_ON = "calendar.on"
    fun on(context: Context) = ProfileStore.prefs(context).getBoolean(K_ON, false)
    fun set(context: Context, v: Boolean) = ProfileStore.prefs(context).edit().putBoolean(K_ON, v).apply()
    fun permitted(context: Context) = ContextCompat.checkSelfPermission(context, Manifest.permission.READ_CALENDAR) == PackageManager.PERMISSION_GRANTED

    /** Appointments in the next `days` days, timed ones only. */
    fun events(context: Context, days: Int = 7): List<CalEvent> {
        if (!permitted(context)) return emptyList()
        val now = System.currentTimeMillis()
        val b = CalendarContract.Instances.CONTENT_URI.buildUpon()
        ContentUris.appendId(b, now)
        ContentUris.appendId(b, now + days * 86_400_000L)
        val out = ArrayList<CalEvent>()
        runCatching {
            context.contentResolver.query(
                b.build(),
                arrayOf(CalendarContract.Instances.EVENT_ID, CalendarContract.Instances.TITLE, CalendarContract.Instances.BEGIN, CalendarContract.Instances.END, CalendarContract.Instances.ALL_DAY),
                null, null, "${CalendarContract.Instances.BEGIN} ASC",
            )?.use { c ->
                while (c.moveToNext()) {
                    if (c.getInt(4) == 1) continue
                    out.add(CalEvent(c.getLong(0), c.getString(1) ?: "Appointment", c.getLong(2), c.getLong(3)))
                }
            }
        }
        return out.take(20)
    }
}

/**
 * Your week's appointments, each set against your day: the ones that fall in a weak
 * hour or an unlucky stretch are marked, with the best hour that day to move them to.
 * The calendar is read on the phone and never stored or sent.
 */
@Composable
fun CalendarScreen(profile: BirthProfile, place: Place, settings: EngineSettings, onBack: () -> Unit, onRemedy: () -> Unit = {}) {
    val context = LocalContext.current
    var allowed by remember { mutableStateOf(CalendarCheck.permitted(context) && CalendarCheck.on(context)) }
    var checks by remember { mutableStateOf<List<CalCheck>?>(null) }
    val launcher = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { ok ->
        CalendarCheck.set(context, ok)
        allowed = ok
    }
    LaunchedEffect(allowed) {
        if (!allowed) return@LaunchedEffect
        val evs = withContext(Dispatchers.IO) { CalendarCheck.events(context) }
        val engine = EngineProvider.get(context)
        val out = ArrayList<CalCheck>()
        for (e in evs) {
            runCatching {
                val at = Instants.fromInstant(Instant.ofEpochMilli(e.begin))
                val m = loadMoment(context, profile, place, settings, at)
                val w = m.windows.firstOrNull { it.caution && e.begin >= it.start.toJava().toEpochMilli() && e.begin < it.end.toJava().toEpochMilli() }
                val dayStart = Instant.ofEpochMilli(e.begin).atZone(ZoneId.systemDefault()).toLocalDate().atStartOfDay(ZoneId.systemDefault()).toInstant()
                val slot = withContext(Dispatchers.Default) {
                    engine.bestSlots(profile.birthInstant(), profile.latitude, profile.longitude, place.latitude, place.longitude, settings.houseSystem, settings.ayanamsa, "meeting", Instants.fromInstant(dayStart), 1u, 1u, com.kshana.app.data.Lang.code)
                }.firstOrNull()
                out.add(
                    CalCheck(
                        e, m.overall, w?.label, m.taraWords,
                        slot?.let { Pair(it.start.toJava().toEpochMilli(), it.end.toJava().toEpochMilli()) }, slot?.score ?: 0.0,
                    ),
                )
            }
        }
        checks = out
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("YOUR CALENDAR", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("Your week's appointments", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "Each meeting, trip or call in your phone's calendar for the next seven days, set against how that hour stands for you. Weak ones get a better hour the same day. The calendar is read on this phone only; nothing is stored or sent.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )
        if (!allowed) {
            GoldButton("CHECK MY CALENDAR", onClick = { launcher.launch(Manifest.permission.READ_CALENDAR) }, modifier = Modifier.fillMaxWidth())
            Text("You can turn this off any time in Settings.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            return@Column
        }
        val list = checks
        if (list == null) {
            Working("reading your week…")
            return@Column
        }
        if (list.isEmpty()) Text("No timed appointments in the next seven days.", fontFamily = FontFamily.Serif, fontSize = 18.sp, color = KshanaColors.TextSoft)
        val weak = list.count { it.score < 44 || it.window != null }
        if (list.isNotEmpty()) Text(
            if (weak == 0) "All ${list.size} sit in reasonable hours." else "$weak of ${list.size} sit in a weak hour. A better hour is suggested for each.",
            fontFamily = FontFamily.Serif, fontSize = 19.sp, lineHeight = 26.sp, color = KshanaColors.Text,
        )
        val hm = DateTimeFormatter.ofPattern("HH:mm")
        val day = DateTimeFormatter.ofPattern("EEE d MMM uuuu")
        val z = ZoneId.systemDefault()
        for (c in list) {
            val bad = c.score < 44 || c.window != null
            Column(Modifier.fillMaxWidth().glass(14.dp).padding(14.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(c.event.title, style = MaterialTheme.typography.titleMedium, color = KshanaColors.Text, modifier = Modifier.weight(1f))
                    Text(c.score.toInt().toString(), fontFamily = FontFamily.Monospace, fontSize = 18.sp, color = scoreColor(c.score))
                }
                Text("${Instant.ofEpochMilli(c.event.begin).atZone(z).format(day)} · ${Instant.ofEpochMilli(c.event.begin).atZone(z).format(hm)}", fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = KshanaColors.Sky)
                Text(
                    when {
                        c.window != null -> "It falls in a ${c.window.lowercase()}: a slower hour. Give it extra preparation, and keep what you begin then simple."
                        c.score < 44 -> "A slower hour for you; give it extra preparation and allow a little more time."
                        c.score >= 56 -> "A good hour for you. Keep it."
                        else -> "An even hour. Fine to keep."
                    },
                    style = MaterialTheme.typography.bodyMedium, color = if (bad) KshanaColors.Ember else KshanaColors.TextSoft,
                )
                Text(c.dayWords.replaceFirstChar { it.uppercase() }.trimEnd('.') + ".", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                val b = c.better
                if (bad && b != null && c.betterScore > c.score) {
                    Text("Better that day: ${Instant.ofEpochMilli(b.first).atZone(z).format(hm)} – ${Instant.ofEpochMilli(b.second).atZone(z).format(hm)} (${c.betterScore.toInt()})", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Leaf)
                } else if (bad) {
                    // No better hour that day: keep it, and carry something good into it.
                    Text("If it must stay: begin with a small kindness that morning, a word of thanks or a little help to someone, and go in unhurried.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                    Text("WHAT HELPS ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold, modifier = Modifier.clickable(onClick = onRemedy).padding(vertical = 4.dp))
                }
                Text(
                    "OPEN IN CALENDAR ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                    modifier = Modifier.clickable {
                        val uri = ContentUris.withAppendedId(CalendarContract.Events.CONTENT_URI, c.event.id)
                        runCatching { context.startActivity(Intent(Intent.ACTION_VIEW).setData(uri)) }
                    }.padding(vertical = 4.dp),
                )
            }
        }
    }
}

private const val DAYS = 60u

/** Eclipses are listed this far ahead: two years, well under the engine's twenty. */
private const val ECLIPSE_DAYS = 731L

private val MONTH = DateTimeFormatter.ofPattern("MMMM uuuu")
private val DAY_SHORT = DateTimeFormatter.ofPattern("EEE d MMM uuuu")

/** The filter chips: a label and the test an observance name must pass. */
private enum class Filter(val label: String, val test: (String) -> Boolean) {
    All("All", { true }),
    Fasting("Fasting", { it.startsWith("Ekadashi") || it.startsWith("Pradosha") }),
    Moon("Moon", { it.contains("Moon") || it.startsWith("Purnima") || it.startsWith("Amavasya") }),
    Sankranti("Sankranti", { it.contains("Sankranti") }),
    Eclipses("Eclipses", { false }),
    Other("Other", { n -> !Fasting.test(n) && !Moon.test(n) && !Sankranti.test(n) }),
}

/** One row of the calendar, an observance or an eclipse, with its instant and the local day it is grouped under. */
private sealed class CalRow(val at: Double, val day: ZonedDateTime) {
    class Obs(val r: ObservanceRecord, zone: ZoneId) : CalRow(r.exact.seconds(), observedLocal(r, zone))
    class Ecl(val r: EclipseRecord, zone: ZoneId) : CalRow(r.greatest.seconds(), Instants.toLocal(r.greatest, zone))
}

/** The observances calendar under its original name and arguments, so either route keeps working. */
@Composable
fun CalendarScreen(here: Place, settings: EngineSettings, onBack: () -> Unit) = ObservancesScreen(here, settings, onBack)

/** Sixty days of observances at the place the phone is, each with its observed day and exact instant, and two years of eclipses. */
@Composable
fun ObservancesScreen(here: Place, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val zone = remember(here) { try { com.kshana.app.data.CityClocks.of(here.zoneId) } catch (e: DateTimeException) { ZoneId.systemDefault() } }
    var records by remember { mutableStateOf<List<ObservanceRecord>>(emptyList()) }
    var filter by remember { mutableStateOf(Filter.All) }
    var status by remember { mutableStateOf("solving $DAYS days of tithis and ingresses…") }
    var error by remember { mutableStateOf<String?>(null) }
    var eclipses by remember { mutableStateOf<List<EclipseRecord>>(emptyList()) }
    var eclipseError by remember { mutableStateOf<String?>(null) }

    // Two years of eclipses with their local circumstances, once per place.
    LaunchedEffect(here) {
        try {
            val engine = EngineProvider.get(context)
            val from = Instants.now()
            val to = Instants.fromInstant(Instant.now().plus(ECLIPSE_DAYS, ChronoUnit.DAYS))
            eclipses = withContext(Dispatchers.Default) { engine.eclipses(from, to, here.latitude, here.longitude) }
                .sortedBy { it.greatest.seconds() }
            eclipseError = null
        } catch (e: Exception) {
            eclipseError = e.message ?: e.toString()
        }
    }

    LaunchedEffect(here, settings) {
        try {
            val engine = EngineProvider.get(context)
            val found = withContext(Dispatchers.Default) {
                engine.calendar(Instants.now(), DAYS, here.latitude, here.longitude, settings.ayanamsa)
            }
            records = found.sortedBy { it.exact.seconds() }
            status = "${found.size} observances · next $DAYS days · ${here.name}"
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
            status = ""
        }
    }

    val shown: List<CalRow> = when (filter) {
        Filter.All -> records.map<ObservanceRecord, CalRow> { CalRow.Obs(it, zone) } + eclipses.map { CalRow.Ecl(it, zone) }
        Filter.Eclipses -> eclipses.map { CalRow.Ecl(it, zone) }
        else -> records.filter { filter.test(it.name) }.map { CalRow.Obs(it, zone) }
    }.sortedBy { it.at }
    val byMonth = shown.groupBy { YearMonth.from(it.day) }.toSortedMap()

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("CALENDAR", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text("Observances", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text("Ekadashi, Pradosha, the full and new Moons, Sankranti. Each with the day it is kept and the instant it is exact.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
        Text(status, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        ErrorLine(error)
        ErrorLine(eclipseError)

        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (f in Filter.values()) Chip(f.label.uppercase(), selected = filter == f, onClick = { filter = f })
        }

        if (filter == Filter.Eclipses) {
            Text("${eclipses.size} eclipses in the next two years · seen from ${here.name}", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
        if (filter == Filter.Eclipses && eclipseError == null && eclipses.isEmpty()) {
            Text("No eclipse in the next two years.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(top = 8.dp))
        } else if (filter != Filter.Eclipses && records.isNotEmpty() && shown.isEmpty()) {
            Text("Nothing of this kind in the next $DAYS days.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(top = 8.dp))
        }

        for ((month, list) in byMonth) {
            SectionLabel(month.atDay(1).format(MONTH))
            for (row in list) when (row) {
                is CalRow.Obs -> ObservanceRow(row.r, zone)
                is CalRow.Ecl -> EclipseRow(row.r, zone, here)
            }
        }

        Spacer(Modifier.height(12.dp))
        Text(
            "The observed day is the civil day whose sunrise the tithi rules; the exact instant is when the sky itself turns. Reminders use the observed sunrise where one exists, otherwise an hour before the instant; for an eclipse, an hour before its first contact here.",
            fontFamily = KshanaFonts.serif, fontStyle = FontStyle.Italic, fontSize = 14.sp, lineHeight = 20.sp, color = KshanaColors.Muted,
        )
    }
}

/** The local civil date an observance is kept on: the observed sunrise when stated, else the exact instant. */
private fun observedLocal(r: ObservanceRecord, zone: ZoneId) = Instants.toLocal(r.observedSunrise ?: r.exact, zone)

@Composable
private fun ObservanceRow(r: ObservanceRecord, zone: ZoneId) {
    val observed = observedLocal(r, zone)
    val sunrise = r.observedSunrise
    val exactLocal = Instants.toLocal(r.exact, zone)
    val remindAt = if (sunrise != null) Instants.toLocal(sunrise, zone).toInstant().toEpochMilli()
    else exactLocal.toInstant().toEpochMilli() - 60L * 60L * 1000L
    val title = if (sunrise != null) "${r.name} today" else "${r.name} in one hour"
    val text = "exact at ${localText(r.exact, DATE_TIME_S, zone)}"

    HorizontalDivider(color = KshanaColors.LineSoft)
    Column(Modifier.fillMaxWidth().padding(vertical = 10.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.Top) {
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(r.name, style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text)
                Text(
                    if (sunrise != null) "observed ${observed.format(DAY_SHORT)}" else observed.format(DAY_SHORT),
                    style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft,
                )
            }
            Spacer(Modifier.width(12.dp))
            RemindButton(key = "calendar:${r.name}:${r.exact.taiSeconds}", atMillis = remindAt, title = title, text = text)
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("exact", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.width(48.dp))
            Text(localText(r.exact, DATE_TIME_S, zone), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky)
        }
        if (r.note.isNotBlank()) Text(r.note, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
    }
}

/** "Total solar eclipse", "Penumbral lunar eclipse": the engine's kind and family as a name. */
private fun eclipseName(r: EclipseRecord): String =
    "${r.kind.replaceFirstChar { it.uppercaseChar() }} ${r.family.lowercase()} eclipse"

/** The local contacts of a solar eclipse, or the global contacts of a lunar one, with their labels. */
private fun eclipseContacts(r: EclipseRecord): List<Pair<String, ExactInstant>> =
    if (r.family == "solar") {
        listOf("C1" to r.localC1, "C2" to r.localC2, "max" to r.localMax, "C3" to r.localC3, "C4" to r.localC4)
            .mapNotNull { (k, v) -> v?.let { k to it } }
    } else {
        listOf("P1", "U1", "U2", "U3", "U4", "P4").zip(r.contacts).mapNotNull { (k, v) -> v?.let { k to it } }
    }

@Composable
private fun EclipseRow(r: EclipseRecord, zone: ZoneId, here: Place) {
    val name = eclipseName(r)
    val solar = r.family == "solar"
    val contacts = eclipseContacts(r)
    val first = contacts.firstOrNull()?.second ?: r.greatest
    val remindAt = Instants.toLocal(first, zone).toInstant().toEpochMilli() - 60L * 60L * 1000L
    val greatestLocal = Instants.toLocal(r.greatest, zone)

    HorizontalDivider(color = KshanaColors.LineSoft)
    Column(Modifier.fillMaxWidth().padding(vertical = 10.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.Top) {
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(name, style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text)
                Text(
                    greatestLocal.format(DAY_SHORT) + if (r.visibleHere) " · visible here" else " · not visible from ${here.name}",
                    style = MaterialTheme.typography.bodyMedium, color = if (r.visibleHere) KshanaColors.Gold else KshanaColors.TextSoft,
                )
            }
            Spacer(Modifier.width(12.dp))
            RemindButton(
                key = "eclipse:${r.family}:${r.greatest.taiSeconds}", atMillis = remindAt,
                title = "$name in one hour", text = "greatest at ${localText(r.greatest, DATE_TIME_S, zone)}",
            )
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("greatest", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.width(64.dp))
            Text(localText(r.greatest, DATE_TIME_S, zone), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky)
        }
        if (r.visibleHere) {
            for ((label, at) in contacts) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(label, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.width(64.dp))
                    Text(localText(at, DATE_TIME_S, zone), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky)
                }
            }
            Text(
                if (solar) "local magnitude %.3f · obscuration %.1f%%".format(r.localMagnitude, r.localObscuration * 100.0)
                else "umbral magnitude %.3f · penumbral %.3f".format(r.magnitude, r.penumbralMagnitude),
                fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Sky,
            )
        } else {
            Text(
                "Not visible from ${here.name}; greatest eclipse magnitude %.3f elsewhere.".format(r.magnitude),
                style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
            )
        }
        Text(
            "gamma %+.4f · Saros %s".format(r.gamma, r.saros?.toString() ?: "—"),
            fontFamily = KshanaFonts.mono, fontSize = 11.sp, color = KshanaColors.Muted,
        )
    }
}
