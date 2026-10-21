package com.kshana.app.data

import android.content.Context

/**
 * Height above sea level. From a hill the sun clears the horizon earlier and sets later:
 * at Munnar (about 1,500 m) sunrise comes some four to five minutes before the sea-level
 * time, at Ooty or Shimla (over 2,000 m) five to seven. The engine lowers the horizon by
 * 1.76′·√(height in m) for every sunrise and sunset at a place whose height it knows.
 *
 * The offline place list (GeoNames towns) carries no heights, so a person enters one by
 * hand; for the best-known hill towns [guess] offers the town's height when the place is
 * picked. [register] tells the engine the heights of every place the app reads for: the
 * birth, the current place, family and partners. It runs when the engine opens and after
 * each of those is saved.
 */
object Heights {
    private data class Hill(val name: String, val lat: Double, val lon: Double, val m: Double)

    /** Hill towns and their usual heights (town centre, rounded), for the hint and the guess. */
    private val hills = listOf(
        Hill("Munnar", 10.0889, 77.0595, 1530.0),
        Hill("Ooty", 11.4102, 76.6950, 2240.0),
        Hill("Kodaikanal", 10.2381, 77.4892, 2130.0),
        Hill("Coonoor", 11.3530, 76.7959, 1850.0),
        Hill("Kotagiri", 11.4218, 76.8612, 1790.0),
        Hill("Yercaud", 11.7748, 78.2097, 1500.0),
        Hill("Valparai", 10.3270, 76.9554, 1190.0),
        Hill("Vagamon", 9.6862, 76.9052, 1100.0),
        Hill("Kalpetta", 11.6085, 76.0830, 780.0),
        Hill("Madikeri", 12.4244, 75.7382, 1150.0),
        Hill("Chikmagalur", 13.3153, 75.7754, 1090.0),
        Hill("Bengaluru", 12.9716, 77.5946, 920.0),
        Hill("Mahabaleshwar", 17.9237, 73.6586, 1350.0),
        Hill("Mount Abu", 24.5926, 72.7156, 1220.0),
        Hill("Pachmarhi", 22.4674, 78.4346, 1070.0),
        Hill("Shimla", 31.1048, 77.1734, 2200.0),
        Hill("Manali", 32.2432, 77.1892, 2050.0),
        Hill("Dharamshala", 32.2190, 76.3234, 1460.0),
        Hill("Dalhousie", 32.5387, 75.9710, 1970.0),
        Hill("Mussoorie", 30.4598, 78.0644, 2000.0),
        Hill("Nainital", 29.3919, 79.4542, 2080.0),
        Hill("Almora", 29.5971, 79.6591, 1640.0),
        Hill("Darjeeling", 27.0410, 88.2663, 2040.0),
        Hill("Gangtok", 27.3389, 88.6065, 1650.0),
        Hill("Shillong", 25.5788, 91.8933, 1520.0),
        Hill("Srinagar", 34.0837, 74.7973, 1590.0),
        Hill("Leh", 34.1526, 77.5771, 3500.0),
        Hill("Kathmandu", 27.7172, 85.3240, 1400.0),
    )

    /** The hint under a height field. */
    const val HINT = "Leave it empty near the sea. In the hills it matters: Munnar is about 1,500 m, Ooty and Shimla over 2,000 m, and there the sun rises a few minutes earlier than at the coast."

    /** The usual height of a known hill town within about 12 km of the place, or 0. */
    fun guess(latitude: Double, longitude: Double): Double =
        hills.filter { kotlin.math.abs(it.lat - latitude) < 0.11 && kotlin.math.abs(it.lon - longitude) < 0.11 }
            .minByOrNull { (it.lat - latitude) * (it.lat - latitude) + (it.lon - longitude) * (it.lon - longitude) }?.m ?: 0.0

    /** "1,530" for a field, "" for none. */
    fun fieldText(m: Double): String = if (m > 0.0) "%.0f".format(m) else ""

    /** A height typed by the person: metres, 0 when empty or not a number. */
    fun parse(text: String): Double = text.trim().replace(",", "").toDoubleOrNull()?.takeIf { it.isFinite() && it > 0.0 }?.coerceAtMost(9000.0) ?: 0.0

    /** Tell the engine every known place height. Safe to call anywhere; never throws. */
    fun register(context: Context) {
        runCatching {
            uniffi.kshana_ffi.clearPlaceElevations()
            val places = buildList {
                ProfileStore.load(context)?.let { add(it.place()) }
                add(ProfileStore.currentLocation(context))
                FamilyStore.family(context).forEach { add(it.place()) }
                ProfileStore.partners(context).forEach { add(it.place()) }
            }
            for (p in places) if (p.elevationM > 0.0) uniffi.kshana_ffi.setPlaceElevation(p.latitude, p.longitude, p.elevationM)
        }
    }
}
