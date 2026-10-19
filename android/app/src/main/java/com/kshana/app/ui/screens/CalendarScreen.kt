package com.kshana.app.ui.screens

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
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.EclipseRecord
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.ObservanceRecord
import java.time.DateTimeException
import java.time.Instant
import java.time.YearMonth
import java.time.ZoneId
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter
import java.time.temporal.ChronoUnit
import com.kshana.app.ui.sky.KshanaFonts

private const val DAYS = 60u

/** Eclipses are listed this far ahead: two years, well under the engine's twenty. */
private const val ECLIPSE_DAYS = 731L

private val MONTH = DateTimeFormatter.ofPattern("MMMM uuuu")
private val DAY_SHORT = DateTimeFormatter.ofPattern("EEE d MMM")

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

/** Sixty days of observances at the place the phone is, each with its observed day and exact instant, and two years of eclipses. */
@Composable
fun CalendarScreen(here: Place, settings: EngineSettings, onBack: () -> Unit) {
    val context = LocalContext.current
    val zone = remember(here) { try { ZoneId.of(here.zoneId) } catch (e: DateTimeException) { ZoneId.systemDefault() } }
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
