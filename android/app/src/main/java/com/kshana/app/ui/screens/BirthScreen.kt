package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
import com.kshana.app.ui.theme.glass
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.CityClocks
import com.kshana.app.data.OldClockCard
import com.kshana.app.data.FamilyStore
import com.kshana.app.data.Place
import com.kshana.app.data.ProfileStore
import com.kshana.app.data.Uncertainty
import com.kshana.app.net.searchPlaces
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.data.Heights
import com.kshana.app.data.NazhikaEntry
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.NazhikaTimeRecord
import java.time.DateTimeException
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import java.util.Locale
import com.kshana.app.ui.sky.KshanaFonts

private data class LevelCopy(val headline: String, val detail: String)

/** Ascendant ≈ 0°15′ per minute of clock time; the odds below follow that rate. */
private val LEVEL_COPY = listOf(
    LevelCopy("The time is exact.", "Every reading is as sharp as it can be, down to the hour."),
    LevelCopy("A little doubt.", "Everything holds; a few hour-by-hour readings may shift slightly."),
    LevelCopy("Some doubt.", "Daily and monthly readings hold; the exact hour of a few turns may move."),
    LevelCopy("Real doubt.", "Most readings hold; some dates may move by a few days."),
    LevelCopy("Large doubt.", "The broad picture holds; dates for some parts of life may be off by weeks."),
    LevelCopy("Very large doubt.", "Only the broad picture holds. Use Find your exact birth time under Explore to sharpen it."),
    LevelCopy("No time at all.", "The broad seasons and your daily luck hold. Dates for each part of life cannot be trusted."),
)

/**
 * "The moment you arrived": date, time, place, stated uncertainty. Saves the profile and
 * hands control back so the caller can navigate to the chart.
 */
@Composable
fun BirthScreen(
    existing: BirthProfile?,
    onCast: (BirthProfile) -> Unit,
    heading: String = "BIRTH",
    title: String = "The moment you arrived",
    intro: String = "Date, clock time and place, as written on the record. Say how sure the time is; the readings are drawn to that width.",
    nameLabel: String = "Name",
    button: String = "SEE MY READING",
    persist: Boolean = true,
    onBack: (() -> Unit)? = null,
) {
    val context = LocalContext.current
    val initialPlace = existing?.place() ?: ProfileStore.kochi

    // A time first entered in nazhika comes back as it was typed: the horoscope's date and count.
    val nz0 = existing?.nazhika
    var name by remember { mutableStateOf(existing?.name ?: "") }
    var year by remember { mutableStateOf((nz0?.year ?: existing?.year)?.toString() ?: "") }
    var month by remember { mutableStateOf((nz0?.month ?: existing?.month)?.toString() ?: "") }
    var day by remember { mutableStateOf((nz0?.day ?: existing?.day)?.toString() ?: "") }
    var hour by remember { mutableStateOf(existing?.hour?.toString() ?: "") }
    var minute by remember { mutableStateOf(existing?.minute?.toString() ?: "") }
    var second by remember { mutableStateOf(existing?.second?.toString() ?: "0") }
    var level by remember { mutableIntStateOf(existing?.uncertaintyLevel ?: 0) }
    var query by remember { mutableStateOf("") }
    var placeName by remember { mutableStateOf(initialPlace.name) }
    var latitude by remember { mutableStateOf("%.4f".format(initialPlace.latitude)) }
    var longitude by remember { mutableStateOf("%.4f".format(initialPlace.longitude)) }
    var zoneId by remember { mutableStateOf(initialPlace.zoneId) }
    // The zone the picked place itself carries, so the old-clock hint can offer to go back to it.
    var placeZone by remember { mutableStateOf(initialPlace.zoneId) }
    var elevation by remember { mutableStateOf(Heights.fieldText(initialPlace.elevationM)) }
    var problem by remember { mutableStateOf<String?>(null) }

    // Nazhika entry: the count, what it is counted from, and the clock time it gives.
    var nzOn by remember { mutableStateOf(nz0 != null) }
    var nzN by remember { mutableStateOf(nz0?.nazhika?.toString() ?: "") }
    var nzV by remember { mutableStateOf(nz0?.vinazhika?.toString() ?: "") }
    var nzSunrise by remember { mutableStateOf(nz0?.fromSunrise ?: true) }
    var nzPrev by remember { mutableStateOf(nz0?.previousDay ?: false) }
    var nzResult by remember { mutableStateOf<NazhikaTimeRecord?>(null) }
    var nzLocal by remember { mutableStateOf<LocalDateTime?>(null) }
    var nzFor by remember { mutableStateOf("") }
    var nzError by remember { mutableStateOf<String?>(null) }
    val nzKey = listOf(nzOn, year, month, day, nzN, nzV, nzSunrise, nzPrev, latitude, longitude, zoneId, elevation, com.kshana.app.data.Lang.code).joinToString("|")
    LaunchedEffect(nzKey) {
        if (!nzOn) { nzResult = null; nzLocal = null; nzError = null; return@LaunchedEffect }
        val y = year.toIntOrNull(); val mo = month.toIntOrNull(); val d = day.toIntOrNull()
        val n = nzN.toIntOrNull(); val v = if (nzV.isBlank()) 0 else nzV.toIntOrNull()
        val lat = latitude.toDoubleOrNull(); val lon = longitude.toDoubleOrNull()
        nzResult = null; nzLocal = null
        if (y == null || mo == null || d == null || n == null || v == null || lat == null || lon == null) { nzError = null; return@LaunchedEffect }
        if (n > 60 || v > 59) { nzError = tr("Nazhika run from 0 to 60 and vinazhika from 0 to 59."); return@LaunchedEffect }
        // The clock's offset on that date (the old city clock where it applies).
        val offset = runCatching {
            val at6 = LocalDateTime.of(y, mo, d, 6, 0)
            CityClocks.zone(zoneId.trim(), at6)?.rules?.getOffset(at6)?.totalSeconds
        }.getOrNull()
        if (offset == null) { nzError = tr("Check the date and the time zone first."); return@LaunchedEffect }
        delay(250)
        try {
            val engine = EngineProvider.get(context)
            val r = withContext(Dispatchers.Default) {
                engine.nazhikaToInstant(y.toLong(), mo.toUInt(), d.toUInt(), offset, lat, lon, Heights.parse(elevation), n.toUInt(), v.toUInt(), nzSunrise, nzPrev, com.kshana.app.data.Lang.code)
            }
            nzResult = r
            nzLocal = Instants.toLocal(r.at, ZoneOffset.ofTotalSeconds(offset)).toLocalDateTime().withNano(0)
            nzFor = nzKey
            nzError = null
        } catch (e: Exception) {
            nzError = if ((e.message ?: "").contains("does not rise")) tr("The sun does not rise at this place on that date, so the count has no start. Use the clock time instead.")
            else tr("That date or count could not be worked out. Check the date and the place.")
        }
    }
    // How sure a nazhika time is: with vinazhika within a minute or two, whole nazhika within a quarter hour.
    // Set when the entry changes, never on opening a saved profile.
    var nzLevelKey by remember { mutableStateOf(nzOn to nzV.isNotBlank()) }
    LaunchedEffect(nzOn, nzV.isNotBlank()) {
        val k = nzOn to nzV.isNotBlank()
        if (k != nzLevelKey) {
            nzLevelKey = k
            if (nzOn) level = NazhikaEntry.uncertaintyLevel(nzV.isNotBlank())
        }
    }

    // Online place search (any place on Earth). Off until the user allows online features.
    val scope = rememberCoroutineScope()
    var onlineOn by remember { mutableStateOf(FamilyStore.onlineAllowed(context)) }
    var earthQuery by remember { mutableStateOf("") }
    var earthResults by remember { mutableStateOf<List<Place>>(emptyList()) }
    var earthError by remember { mutableStateOf<String?>(null) }
    var searching by remember { mutableStateOf(false) }

    val matches = remember(query) {
        if (query.isBlank()) emptyList()
        else ProfileStore.places.filter { it.name.contains(query.trim(), ignoreCase = true) }.take(6)
    }

    fun choose(p: Place) {
        placeName = p.name
        latitude = "%.4f".format(p.latitude)
        longitude = "%.4f".format(p.longitude)
        zoneId = p.zoneId
        placeZone = p.zoneId
        // A known hill town brings its height; any other place starts empty (sea level).
        elevation = Heights.fieldText(if (p.elevationM > 0.0) p.elevationM else Heights.guess(p.latitude, p.longitude))
        query = ""
        earthResults = emptyList()
        earthError = null
    }

    fun searchEarth() {
        val q = earthQuery.trim()
        if (q.isEmpty() || searching) return
        scope.launch {
            searching = true
            try {
                val found = searchPlaces(context, q)
                earthResults = found
                earthError = if (found.isEmpty()) "No place found for \"$q\"." else null
            } catch (e: Exception) {
                earthResults = emptyList()
                earthError = e.message ?: e.toString()
            }
            searching = false
        }
    }

    fun build(): BirthProfile? {
        val ty = year.toIntOrNull(); val tmo = month.toIntOrNull(); val td = day.toIntOrNull()
        val lat = latitude.toDoubleOrNull(); val lon = longitude.toDoubleOrNull()
        if (ty == null || tmo == null || td == null) { problem = "The date needs a year, a month and a day."; return null }
        // In nazhika the clock date and time come from the count (past midnight, the date moves on).
        val fromCount = if (nzOn) {
            val at = nzLocal
            if (nzN.toIntOrNull() == null) { problem = tr("Enter the nazhika from the horoscope, or switch back to the clock time."); return null }
            if (at == null || nzFor != nzKey) { problem = nzError ?: tr("Working out the clock time. Try again in a moment."); return null }
            at
        } else null
        val y = fromCount?.year ?: ty; val mo = fromCount?.monthValue ?: tmo; val d = fromCount?.dayOfMonth ?: td
        val h = fromCount?.hour ?: hour.toIntOrNull(); val mi = fromCount?.minute ?: minute.toIntOrNull()
        val s = fromCount?.second ?: (second.toIntOrNull() ?: 0)
        if (h == null || mi == null) { problem = "The time needs an hour and a minute. If it is unknown, enter 12:00 and set the slider to unknown."; return null }
        if (lat == null || lon == null || lat < -90 || lat > 90 || lon < -180 || lon > 180) { problem = "Latitude must be −90…90 and longitude −180…180."; return null }
        val local = try { LocalDateTime.of(y, mo, d, h, mi, s) } catch (e: DateTimeException) { problem = "That date and time do not exist on the calendar."; return null }
        val zone = CityClocks.zone(zoneId.trim(), local) ?: run { problem = "Unknown time zone id. Use an IANA name such as Asia/Kolkata."; return null }
        // A clock time inside a summer-time change is either skipped (it never happened) or
        // happened twice; java.time would silently move or pick one, an hour off the birth.
        val offsets = zone.rules.getValidOffsets(local)
        if (offsets.isEmpty()) {
            problem = "In ${zone.id} the clocks jumped forward over that time, so it never showed on a clock there. Check the hour on the record."
            return null
        }
        if (offsets.size > 1) {
            problem = "In ${zone.id} the clocks went back and that time happened twice, an hour apart. Replace the time zone with the offset on the record: ${offsets.joinToString(" or ") { "UTC" + it.id }}."
            return null
        }
        problem = null
        return BirthProfile(
            name = name.trim().ifEmpty { if (persist) "You" else "Partner" },
            year = y, month = mo, day = d, hour = h, minute = mi, second = s,
            uncertaintyLevel = level,
            latitude = lat, longitude = lon,
            placeName = placeName.trim().ifEmpty { "%.2f, %.2f".format(lat, lon) },
            zoneId = zoneId.trim(),
            elevationM = Heights.parse(elevation),
            nazhika = if (nzOn) NazhikaEntry(ty, tmo, td, nzN.toInt(), nzV.toIntOrNull(), nzSunrise, nzPrev) else null,
        )
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 40.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text(heading, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            if (onBack != null) Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text(title, style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(intro, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)

        Field(name, { name = it }, nameLabel, Modifier.fillMaxWidth())

        SectionLabel("Date")
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            NumberField(year, { year = it.filter(Char::isDigit).take(4) }, "Year", Modifier.weight(1.4f))
            NumberField(month, { month = it.filter(Char::isDigit).take(2) }, "Month", Modifier.weight(1f))
            NumberField(day, { day = it.filter(Char::isDigit).take(2) }, "Day", Modifier.weight(1f))
        }

        SectionLabel(if (nzOn) tr("Time in nazhika") else tr("Clock time"))
        OptionBox(tr("My horoscope gives the time in nazhika"), selected = nzOn, onClick = {
            // Going back to the clock keeps the time the count gave.
            if (nzOn) nzLocal?.let { hour = it.hour.toString(); minute = it.minute.toString(); second = it.second.toString() }
            nzOn = !nzOn
        }, modifier = Modifier.fillMaxWidth())
        if (!nzOn) {
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                NumberField(hour, { hour = it.filter(Char::isDigit).take(2) }, "Hour (0–23)", Modifier.weight(1f))
                NumberField(minute, { minute = it.filter(Char::isDigit).take(2) }, "Minute", Modifier.weight(1f))
                NumberField(second, { second = it.filter(Char::isDigit).take(2) }, "Second", Modifier.weight(1f))
            }
        } else {
            NazhikaEntryFields(
                nazhika = nzN, onNazhika = { nzN = it.filter(Char::isDigit).take(2) },
                vinazhika = nzV, onVinazhika = { nzV = it.filter(Char::isDigit).take(2) },
                fromSunrise = nzSunrise, onFromSunrise = { nzSunrise = it },
                previousDay = nzPrev, onPreviousDay = { nzPrev = it },
                typedDate = runCatching { LocalDate.of(year.toInt(), month.toInt(), day.toInt()) }.getOrNull(),
                result = nzResult, local = nzLocal, error = nzError,
            )
        }

        SectionLabel("Place")
        PlacePicker(onPick = { choose(it) })
        Text(
            "Pick the town or village itself, not just the nearest city: every place has its own coordinates, and they set the exact rising sign and hours.",
            style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
        )
        if (matches.isNotEmpty()) {
            Column(Modifier.fillMaxWidth()) {
                for (p in matches) {
                    HorizontalDivider(color = KshanaColors.LineSoft)
                    Row(
                        Modifier.fillMaxWidth().heightIn(min = 44.dp).clickable { choose(p) }.padding(vertical = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(p.name, fontFamily = KshanaFonts.serif, fontSize = 17.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
                        Text("%.2f° %.2f°".format(p.latitude, p.longitude), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
                    }
                }
                HorizontalDivider(color = KshanaColors.LineSoft)
            }
        }

        SectionLabel("Any place on Earth")
        if (!onlineOn) {
            Text("Online place search is off", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            QuietButton("ALLOW ONLINE FEATURES", onClick = {
                FamilyStore.saveOnlineAllowed(context, true)
                onlineOn = true
            })
        } else {
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.CenterVertically) {
                Field(earthQuery, { earthQuery = it }, "Town, city or village", Modifier.weight(1f))
                QuietButton(if (searching) "SEARCHING…" else "SEARCH", onClick = { searchEarth() })
            }
            Text("Looked up on OpenStreetMap; the time zone is derived from the country and longitude. Check it below.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            ErrorLine(earthError)
            if (earthResults.isNotEmpty()) {
                Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    for (p in earthResults) {
                        Chip("${p.name}  ·  ${"%.2f".format(p.latitude)}° ${"%.2f".format(p.longitude)}°", selected = false, onClick = { choose(p) })
                    }
                }
            }
        }

        Text(placeName, fontFamily = KshanaFonts.serif, fontSize = 20.sp, color = KshanaColors.Text)
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            NumberField(latitude, { latitude = it.filter { c -> c.isDigit() || c == '.' || c == '-' } }, "Latitude °N", Modifier.weight(1f), decimal = true)
            NumberField(longitude, { longitude = it.filter { c -> c.isDigit() || c == '.' || c == '-' } }, "Longitude °E", Modifier.weight(1f), decimal = true)
        }
        NumberField(elevation, { elevation = it.filter(Char::isDigit).take(4) }, tr("Height above sea level, m (optional)"), Modifier.fillMaxWidth())
        Text(tr(Heights.HINT), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        Field(zoneId, { zoneId = it }, "Time zone (IANA id)", Modifier.fillMaxWidth())
        OldCityClock(year.toIntOrNull(), latitude.toDoubleOrNull(), longitude.toDoubleOrNull(), zoneId, placeZone) { zoneId = it }
        Field(placeName, { placeName = it }, "Place name", Modifier.fillMaxWidth())

        SectionLabel("How sure is the time?")
        Slider(
            value = level.toFloat(),
            onValueChange = { level = it.toInt().coerceIn(0, 6) },
            valueRange = 0f..6f,
            steps = 5,
            colors = SliderDefaults.colors(
                thumbColor = KshanaColors.Gold,
                activeTrackColor = KshanaColors.Gold,
                inactiveTrackColor = KshanaColors.Line,
                activeTickColor = KshanaColors.Night,
                inactiveTickColor = KshanaColors.Muted,
            ),
            modifier = Modifier.fillMaxWidth().height(44.dp),
        )
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            for ((i, l) in Uncertainty.labels.withIndex()) {
                Text(
                    l,
                    fontFamily = KshanaFonts.mono,
                    fontSize = 9.sp,
                    color = if (i == level) KshanaColors.Gold else KshanaColors.Muted,
                    modifier = Modifier.clickable { level = i }.padding(vertical = 4.dp),
                )
            }
        }
        Text(LEVEL_COPY[level].headline, style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text)
        Text(LEVEL_COPY[level].detail, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)

        Spacer(Modifier.height(8.dp))
        problem?.let { Text(it, color = KshanaColors.Sky, style = MaterialTheme.typography.bodyMedium) }
        GoldButton(button, onClick = {
            val p = build()
            if (p != null) {
                if (persist) ProfileStore.save(context, p)
                onCast(p)
            }
        }, modifier = Modifier.fillMaxWidth())
        Text("Everything is worked out on this phone. Nothing leaves it.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(top = 4.dp))
    }
}

private val NZ_TIME = DateTimeFormatter.ofPattern("h:mm:ss a", Locale.ENGLISH)
private val NZ_DATE = DateTimeFormatter.ofPattern("d MMMM yyyy", Locale.ENGLISH)

private fun nzTime(t: LocalDateTime): String = t.format(NZ_TIME).replace("AM", "am").replace("PM", "pm")

/**
 * The time as a Kerala or Tamil horoscope writes it: nazhika and vinazhika from that
 * morning's sunrise (or from six by the clock). The clock time it gives is shown as the
 * person types, with its date, since a count past midnight runs into the next date.
 *
 * A horoscope's day runs from one sunrise to the next, so a birth after midnight and
 * before sunrise is counted from the previous morning. The date stays as typed and the box
 * "born after midnight, before sunrise" moves the start back a day. Which date a horoscope
 * writes for such a birth varies, so the app never ticks the box itself: when the count
 * runs past midnight (or, ticked, lands before it) it says so in words and shows the date.
 */
@Composable
private fun NazhikaEntryFields(
    nazhika: String, onNazhika: (String) -> Unit,
    vinazhika: String, onVinazhika: (String) -> Unit,
    fromSunrise: Boolean, onFromSunrise: (Boolean) -> Unit,
    previousDay: Boolean, onPreviousDay: (Boolean) -> Unit,
    typedDate: LocalDate?,
    result: NazhikaTimeRecord?, local: LocalDateTime?, error: String?,
) {
    Text(
        tr("A nazhika is 24 minutes and a vinazhika 24 seconds, counted from that morning's sunrise at the birth place. The date above is the date on the horoscope."),
        style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
    )
    Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        NumberField(nazhika, onNazhika, tr("Nazhika (0–60)"), Modifier.weight(1f))
        NumberField(vinazhika, onVinazhika, tr("Vinazhika (if given)"), Modifier.weight(1f))
    }
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Chip(tr("FROM SUNRISE"), selected = fromSunrise, onClick = { onFromSunrise(true) })
        Chip(tr("FROM 6 AM"), selected = !fromSunrise, onClick = { onFromSunrise(false) })
    }
    Text(
        if (fromSunrise) tr("Most horoscopes count from the actual sunrise of the day. The app finds that sunrise for the date and place.")
        else tr("Some horoscopes count from 6 o'clock in the morning by the clock instead. Use this only if yours says so."),
        style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
    )
    OptionBox(tr("Born after midnight, before sunrise"), selected = previousDay, onClick = { onPreviousDay(!previousDay) }, modifier = Modifier.fillMaxWidth())
    Text(
        tr("A horoscope's day runs from one sunrise to the next. A birth in the small hours belongs to the day that began the morning before, so the count starts from that sunrise. Keep the date above as the birth date on the calendar."),
        style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
    )
    ErrorLine(error)
    if (result != null && local != null) {
        Column(
            Modifier.fillMaxWidth().glass(12.dp).padding(12.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Text(trf("That is {0}", nzTime(local)), fontFamily = KshanaFonts.serif, fontSize = 20.sp, color = KshanaColors.Text)
            Text(trf("on {0}", local.toLocalDate().format(NZ_DATE)), fontFamily = KshanaFonts.mono, fontSize = 12.sp, color = KshanaColors.Sky)
            Text(result.note, style = MaterialTheme.typography.bodySmall, color = KshanaColors.TextSoft)
            val hint = when {
                result.pastNextSunrise ->
                    tr("That is more than the whole day holds: from this sunrise to the next is %.1f nazhika. Check the count on the horoscope.").format(result.wholeDayNazhika)
                typedDate != null && !previousDay && local.toLocalDate().isAfter(typedDate) ->
                    trf("This count runs past midnight, into {0}. If {1} is the birth date on the calendar, the birth was after midnight and before sunrise: tick the box above. If the horoscope writes the date of the morning the count began, leave it as it is.", local.toLocalDate().format(NZ_DATE), typedDate.format(NZ_DATE))
                typedDate != null && previousDay && local.toLocalDate().isBefore(typedDate) ->
                    trf("Counted from the sunrise of the day before, this lands on the evening of {0}, before midnight. Untick the box unless the birth was between midnight and sunrise.", local.toLocalDate().format(NZ_DATE))
                result.afterSunset ->
                    tr("A night birth: the daylight that day lasted %.1f nazhika and this count is past it.").format(result.daytimeNazhika)
                else -> null
            }
            if (hint != null) Text(hint, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Gold)
            Text(tr("This clock time is what gets saved and read."), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
    }
}

@Composable
internal fun Field(value: String, onChange: (String) -> Unit, label: String, modifier: Modifier = Modifier) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label, style = MaterialTheme.typography.labelSmall) },
        singleLine = true,
        textStyle = TextStyle(fontFamily = KshanaFonts.sans, fontSize = 15.sp, color = KshanaColors.Text),
        colors = fieldColors(),
        modifier = modifier,
    )
}

@Composable
private fun NumberField(value: String, onChange: (String) -> Unit, label: String, modifier: Modifier = Modifier, decimal: Boolean = false) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label, style = MaterialTheme.typography.labelSmall) },
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = if (decimal) KeyboardType.Decimal else KeyboardType.Number),
        textStyle = TextStyle(fontFamily = KshanaFonts.mono, fontSize = 15.sp, color = KshanaColors.Text),
        colors = fieldColors(),
        modifier = modifier,
    )
}

@Composable
private fun fieldColors() = OutlinedTextFieldDefaults.colors(
    focusedTextColor = KshanaColors.Text,
    unfocusedTextColor = KshanaColors.Text,
    cursorColor = KshanaColors.Gold,
    focusedBorderColor = KshanaColors.Gold,
    unfocusedBorderColor = KshanaColors.Line,
    focusedLabelColor = KshanaColors.Gold,
    unfocusedLabelColor = KshanaColors.Muted,
    focusedContainerColor = KshanaColors.Surface,
    unfocusedContainerColor = KshanaColors.Surface,
)

/**
 * Old clocks. Bombay kept its own clock (Bombay Time, UTC+04:51) until 1955 and Calcutta its
 * own (Calcutta Time, UTC+05:53:20) until 1948, while the time zone database files both
 * under Indian time from 1906 ([CityClocks] has the sources). Places in Greater Bombay and
 * Calcutta now carry the city-clock zone ("Kshana/Bombay", "Kshana/Calcutta") by default, which
 * is the city clock before the switch and IST after it; this card says which clock is in use
 * and lets the person change it, since many records of the time were already written in IST.
 * For records before 1906, when towns kept the sun's own time, it also offers local mean time
 * (longitude / 15 hours).
 */
@Composable
private fun OldCityClock(year: Int?, lat: Double?, lon: Double?, zone: String, placeZone: String, onUse: (String) -> Unit) {
    if (year == null || lat == null || lon == null) return
    val card = OldClockCard.of(year, lat, lon, zone, placeZone) ?: return
    Column(
        Modifier.fillMaxWidth().glass(12.dp).padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        for (l in card.lines) Text(l, style = MaterialTheme.typography.bodySmall, color = KshanaColors.TextSoft)
        for ((label, id) in card.actions) {
            Text(
                label,
                style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
                modifier = Modifier.clickable { onUse(id) }.padding(vertical = 4.dp),
            )
        }
    }
}
