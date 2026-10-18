# Kshana on Google Play

Everything needed to publish: the listing text, the forms, the permissions, and the steps.

## Listing

**App name:** Kshana: Your Day, Plainly

**Short description (80 characters max):**
Your day, week and year in plain words, with what to do. All on your phone.

**Full description:**

Kshana tells you how today, this week, this month and this year look for the parts of
your life that matter: love, work, money, health, family, study, home, travel and children.
Every line is plain English: what is likely, when, and what to do about it. No charts to
read and no jargon.

What you get:
- Your day at a glance, with the good hours and the hours to go easy, counted down.
- Ahead: today, this week, this month and this year, for each part of your life.
- Ask: will it happen, when, is now a good time, should I, will it come back. A straight
  answer with dates, and how sure it is.
- Us: how two people match, how things stand between you now and over the coming months,
  and the best time to talk.
- What to do: for each difficult stretch, the cause in plain words, how long it lasts, and
  what helps. Deeds anyone can do, giving, habits, nature and quiet time, with prayer and
  temple practice offered separately for those who want them.
- The month's letter, a look back at any day of your life, and the book of your life.
- Your record: say whether past readings came true, and see how often the app was right.
- Your family on one screen, shared phone to phone with a link or QR code.
- Reminders you choose, a sunrise reading, and a home-screen widget.

Private by design: every reading is worked out on your phone. No account, no ads, no
data collected. The app works offline.

Kshana offers guidance, not a guarantee. It does not replace medical, legal, financial
or mental-health advice. If you are struggling, in India you can call Tele-MANAS on 14416,
free, at any hour.

**Category:** Lifestyle
**Tags:** daily guidance, horoscope, panchang, compatibility
**Contact email:** vysakhsuresh200290@gmail.com
**Privacy policy URL:** host docs/PRIVACY.md at a public URL (for example a GitHub Pages
page or a Google Sites page) and paste that URL in Play Console. The same text is inside
the app under Settings.

## Content rating (IARC questionnaire)

- Violence, sexuality, language, controlled substances, gambling: none.
- User-generated content shared with others: no. People are shared only phone to phone
  by the person's own choice; there is no in-app communication between users.
- Shares location: no. The app uses a place the person types; it does not read GPS.
- Digital purchases: none at launch.
- Expected rating: Everyone / PEGI 3 equivalent. Target audience is set to 18+ below.

## Target audience and content

- Target age groups: 18 and over.
- Not designed for children; no ads.
- News app: no. Health app: no (it gives general wellbeing guidance only and states that
  it is not medical advice).

## Data safety form

- **Does the app collect or share any of the required user data types?** No.
  Everything stays on the device and never leaves it.
- **Is all user data encrypted in transit?** Not applicable: the app sends no data.
- **Can users request that data be deleted?** Yes: clear the app's data or uninstall,
  and delete the backup file in Downloads/Kshana.
- Notes for reviewers: the backup file Downloads/Kshana/kshana-backup.json is written on
  the device only, can be turned off in Settings, and is never uploaded. Calendar events
  are read on the device only when the person turns on the calendar check and are not
  stored. Voice ask uses the phone's speech recogniser and keeps no audio.

## Permissions and their justification

| Permission | Why | When it is asked |
|---|---|---|
| POST_NOTIFICATIONS | Reminders, the sunrise reading and the evening "did it happen?" | When the person turns on a reminder or the sunrise reading |
| SCHEDULE_EXACT_ALARM | Reminders at the exact minute the person chose (for example the start of a lucky hour). Optional: without it reminders arrive a few minutes late. USE_EXACT_ALARM is not used. | From the reminder settings, by the system page |
| RECEIVE_BOOT_COMPLETED | Re-arm the person's own reminders after a restart | Not asked (normal permission) |
| VIBRATE | A soft tap when the hour turns | Not asked |
| READ_CALENDAR | Optional calendar check: flags the person's own events that fall in a difficult stretch and suggests a better hour. Read only, on the device, not stored. | Only when the person turns the calendar check on |
| RECORD_AUDIO | Optional voice ask: the phone's speech service turns the spoken question into text | Only when the person taps the microphone |
| WRITE_EXTERNAL_STORAGE (max SDK 28) | Write the person's own backup file to Downloads on Android 8 and 9 only | On those versions, when backup runs |

Queries declared: UPI apps (to open a donation the person chooses), speech recognition and
text-to-speech services, and share targets.

## Release steps

1. Build the native engine for the phone (16 KB pages are set in `.cargo/config.toml`):
   `cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o android/app/src/main/jniLibs build --release -p kshana-ffi`
   (on this PC add `CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_OPT_LEVEL=3` if Smart App Control
   blocks a build script).
2. Build the bundle: `cd android && ./gradlew :app:bundleRelease`.
   Output: `android/app/build/outputs/bundle/release/app-release.aab`.
3. The upload key is `android/kshana-upload.jks` (alias `kshana-upload`); its passwords are
   in `android/keystore.properties`. Both are kept out of git. **Back them up somewhere
   safe**: losing the upload key means asking Google for an upload-key reset.
4. In Play Console: create the app, enrol in Play App Signing (Google holds the app signing
   key; you upload with the upload key), upload the .aab to an internal testing track first,
   complete the forms above, then promote to production.
5. Raise `versionCode` in `android/app/build.gradle.kts` for every upload.

## Store assets checklist

- App icon: 512 × 512 PNG, 32-bit, under 1 MB.
- Feature graphic: 1024 × 500 PNG or JPG, no transparency.
- Phone screenshots: 2 to 8, 16:9 or 9:16, each side between 320 and 3840 px
  (for example 1080 × 2400 captures of Now, Ahead, Ask, Us, What to do, the letter).
- Optional: 7-inch and 10-inch tablet screenshots.
- Optional: a promo video on YouTube.
