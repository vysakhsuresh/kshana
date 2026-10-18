package com.kshana.app.system

import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.LinearGradient
import android.graphics.Paint
import android.graphics.RadialGradient
import android.graphics.Shader
import android.graphics.Typeface
import android.text.Layout
import android.text.StaticLayout
import android.text.TextPaint
import androidx.core.content.FileProvider
import java.io.File
import kotlin.random.Random

/**
 * A card to share: one plain sentence on the night sky, with its date. Drawn on the
 * phone as a picture (1080 × 1350, the size messaging apps show whole) and handed to
 * the share sheet. No birth details are on it unless the person typed them.
 */
object ShareCard {
    private const val W = 1080
    private const val H = 1350

    fun render(context: Context, label: String, sentence: String, footer: String): Bitmap {
        val bmp = Bitmap.createBitmap(W, H, Bitmap.Config.ARGB_8888)
        val c = Canvas(bmp)
        val night = 0xFF07091A.toInt()
        val gold = 0xFFE3B96A.toInt()
        val text = 0xFFEFE9DC.toInt()
        val muted = 0xFF9A9EB0.toInt()
        val bg = Paint().apply { shader = LinearGradient(0f, 0f, 0f, H.toFloat(), 0xFF151B3A.toInt(), night, Shader.TileMode.CLAMP) }
        c.drawRect(0f, 0f, W.toFloat(), H.toFloat(), bg)
        val glow = Paint().apply { shader = RadialGradient(W * 0.85f, H * 0.1f, W * 0.9f, 0x22E3B96A, 0x00000000, Shader.TileMode.CLAMP) }
        c.drawRect(0f, 0f, W.toFloat(), H.toFloat(), glow)
        val r = Random(sentence.hashCode())
        val star = Paint(Paint.ANTI_ALIAS_FLAG)
        repeat(160) {
            star.color = (((40 + r.nextInt(120)) shl 24) or 0xEFE9DC)
            c.drawCircle(r.nextFloat() * W, r.nextFloat() * H, 1f + r.nextFloat() * 2.2f, star)
        }
        val serif = Typeface.create(Typeface.SERIF, Typeface.NORMAL)
        val small = TextPaint(Paint.ANTI_ALIAS_FLAG).apply { color = gold; textSize = 34f; letterSpacing = 0.18f; typeface = Typeface.create(Typeface.SANS_SERIF, Typeface.BOLD) }
        c.drawText("K S H A N A", 96f, 150f, small)
        val hair = Paint().apply { shader = LinearGradient(96f, 0f, 520f, 0f, gold, 0x00E3B96A, Shader.TileMode.CLAMP); strokeWidth = 2f }
        c.drawLine(96f, 182f, 520f, 182f, hair)
        small.color = muted
        small.textSize = 30f
        c.drawText(label.uppercase(), 96f, 250f, small)
        val body = TextPaint(Paint.ANTI_ALIAS_FLAG).apply { color = text; typeface = serif; textSize = if (sentence.length > 220) 52f else 64f }
        val layout = StaticLayout.Builder.obtain(sentence, 0, sentence.length, body, W - 192)
            .setAlignment(Layout.Alignment.ALIGN_NORMAL).setLineSpacing(14f, 1f).build()
        c.save()
        c.translate(96f, 330f)
        layout.draw(c)
        c.restore()
        val foot = TextPaint(Paint.ANTI_ALIAS_FLAG).apply { color = muted; textSize = 28f; typeface = Typeface.create(Typeface.SANS_SERIF, Typeface.NORMAL) }
        c.drawText(footer, 96f, H - 110f, foot)
        foot.color = gold
        c.drawText("Kshana · worked out on the phone, nothing leaves it", 96f, H - 66f, foot)
        return bmp
    }

    /** Render and open the share sheet with the picture and the sentence as text. */
    fun share(context: Context, label: String, sentence: String, footer: String) {
        runCatching {
            val bmp = render(context, label, sentence, footer)
            val dir = File(context.cacheDir, "share").apply { mkdirs() }
            val f = File(dir, "kshana-card.png")
            f.outputStream().use { bmp.compress(Bitmap.CompressFormat.PNG, 100, it) }
            val uri = FileProvider.getUriForFile(context, "${context.packageName}.files", f)
            val i = Intent(Intent.ACTION_SEND).setType("image/png")
                .putExtra(Intent.EXTRA_STREAM, uri)
                .putExtra(Intent.EXTRA_TEXT, "$sentence\n\n— Kshana")
                .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            context.startActivity(Intent.createChooser(i, "Share").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        }
    }
}
