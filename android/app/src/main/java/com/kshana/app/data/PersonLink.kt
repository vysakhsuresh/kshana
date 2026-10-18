package com.kshana.app.data

import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Color
import android.net.Uri
import com.google.zxing.BarcodeFormat
import com.google.zxing.EncodeHintType
import com.google.zxing.qrcode.QRCodeWriter
import com.google.zxing.qrcode.decoder.ErrorCorrectionLevel

/**
 * A person's birth details as a link: kshana://person?n=…&d=1997-09-22&t=18:42:00&u=3
 * &la=9.9312&lo=76.2673&p=Kochi&z=Asia/Kolkata. Shared by message or shown as a QR code;
 * opening it (or pasting it) adds the person under Us. Only what the sender chose is in
 * it; nothing goes through a server.
 */
object PersonLink {
    fun of(p: BirthProfile): String = Uri.Builder().scheme("kshana").authority("person")
        .appendQueryParameter("n", p.name)
        .appendQueryParameter("d", "%04d-%02d-%02d".format(p.year, p.month, p.day))
        .appendQueryParameter("t", "%02d:%02d:%02d".format(p.hour, p.minute, p.second))
        .appendQueryParameter("u", p.uncertaintyLevel.toString())
        .appendQueryParameter("la", "%.4f".format(java.util.Locale.ROOT, p.latitude))
        .appendQueryParameter("lo", "%.4f".format(java.util.Locale.ROOT, p.longitude))
        .appendQueryParameter("p", p.placeName)
        .appendQueryParameter("z", p.zoneId)
        .build().toString()

    /** Parse a link, or the first kshana:// link inside any text (a pasted message). */
    fun parse(text: String?): BirthProfile? {
        if (text == null) return null
        val i = text.indexOf("kshana://person")
        if (i < 0) return null
        val raw = text.substring(i).split(Regex("\\s")).first()
        return runCatching {
            val u = Uri.parse(raw)
            val d = u.getQueryParameter("d")!!.split("-").map { it.toInt() }
            val t = (u.getQueryParameter("t") ?: "12:00:00").split(":").map { it.toInt() }
            BirthProfile(
                name = u.getQueryParameter("n") ?: "Someone",
                year = d[0], month = d[1], day = d[2],
                hour = t.getOrElse(0) { 12 }, minute = t.getOrElse(1) { 0 }, second = t.getOrElse(2) { 0 },
                uncertaintyLevel = (u.getQueryParameter("u") ?: "3").toInt().coerceIn(0, 6),
                latitude = u.getQueryParameter("la")!!.toDouble(),
                longitude = u.getQueryParameter("lo")!!.toDouble(),
                placeName = u.getQueryParameter("p") ?: "",
                zoneId = u.getQueryParameter("z") ?: "Asia/Kolkata",
            )
        }.getOrNull()
    }

    fun shareIntent(p: BirthProfile, fromName: String): Intent {
        val text = "${fromName.ifBlank { "Someone" }} shared ${p.name.substringBefore(' ')}'s birth details with you for Kshana. " +
            "Open Kshana › Us › + ADD › Paste a shared person, with this message copied:\n\n${of(p)}"
        return Intent.createChooser(Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, text), "Share")
    }

    fun qr(text: String, size: Int = 720): Bitmap? = runCatching {
        val m = QRCodeWriter().encode(text, BarcodeFormat.QR_CODE, size, size, mapOf(EncodeHintType.ERROR_CORRECTION to ErrorCorrectionLevel.M, EncodeHintType.MARGIN to 1))
        val bmp = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888)
        for (x in 0 until size) for (y in 0 until size) bmp.setPixel(x, y, if (m[x, y]) Color.BLACK else Color.WHITE)
        bmp
    }.getOrNull()

    /** Add a parsed person under Us, unless one with the same name and birth is there. */
    fun add(context: Context, p: BirthProfile): Boolean {
        val have = ProfileStore.partners(context).any { it.name == p.name && it.year == p.year && it.month == p.month && it.day == p.day }
        if (!have) ProfileStore.addPartner(context, p)
        return !have
    }
}
