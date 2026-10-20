/* Swiss Ephemeris 2.10.03 oracle for Kshana golden values.
 *
 * Build (any C99 compiler; sources from https://github.com/aloistr/swisseph tag v2.10.03,
 * ephemeris files semo_18.se1, sepl_18.se1 and sefstars.txt from its ephe/ directory):
 *
 *   gcc -O2 -I<swisseph> -o oracle swisseph_oracle.c <swisseph>/sweph.c
 *       <swisseph>/swephlib.c <swisseph>/swedate.c <swisseph>/swejpl.c
 *       <swisseph>/swemmoon.c <swisseph>/swemplan.c <swisseph>/swehouse.c
 *       <swisseph>/swecl.c <swisseph>/swehel.c -lm
 *
 * Run:
 *   oracle aya  <ephe-dir> swi > tests/golden/swisseph_ayanamsa.json
 *   oracle node <ephe-dir> swi   Rahu's sidereal (Lahiri) sign ingresses 2023-2026, true
 *                                and mean node (pinned in the kshana-forecast gochara tests)
 * Without the trailing "swi" the built-in Moshier ephemeris is used instead.
 */
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "swephexp.h"

static int EPH = SEFLG_MOSEPH;

static const int MODES[] = {0, 1, 3, 5, 7, 21, 22, 25, 26, 27, 28, 29, 43, 44, 45, 46};
#define NMODES (sizeof(MODES) / sizeof(MODES[0]))

static void aya(int y, int m, int d, int h, int mi, double s, int last) {
  double jd[2], daya;
  char serr[256];
  swe_utc_to_jd(y, m, d, h, mi, s, SE_GREG_CAL, jd, serr);
  printf("  {\"utc\": \"%04d-%02d-%02dT%02d:%02d:%06.3f\", \"jd_tt\": %.10f, \"jd_ut\": %.10f",
         y, m, d, h, mi, s, jd[0], jd[1]);
  for (unsigned k = 0; k < NMODES; k++) {
    swe_set_sid_mode(MODES[k], 0, 0);
    if (swe_get_ayanamsa_ex_ut(jd[1], EPH, &daya, serr) < 0) {
      fprintf(stderr, "%s\n", serr);
      exit(1);
    }
    printf(", \"%d\": %.12f", MODES[k], daya);
  }
  printf("}%s\n", last ? "" : ",");
}

static double sid_node(double jd_ut, int body) {
  double x[6];
  char serr[256];
  swe_set_sid_mode(SE_SIDM_LAHIRI, 0, 0);
  if (swe_calc_ut(jd_ut, body, EPH | SEFLG_SIDEREAL, x, serr) < 0) {
    fprintf(stderr, "%s\n", serr);
    exit(1);
  }
  return x[0];
}

static void print_ut(double jd_ut) {
  int y, m, d, h, mi;
  double s;
  swe_jdut1_to_utc(jd_ut, SE_GREG_CAL, &y, &m, &d, &h, &mi, &s);
  printf("%04d-%02d-%02dT%02d:%02d:%06.3f", y, m, d, h, mi, s);
}

/* every sidereal sign change of the node between two UT dates: hourly scan + bisection */
static void ingresses(int body, double from, double to) {
  double step = 1.0 / 24.0;
  double a = from;
  int sa = (int)(sid_node(a, body) / 30.0);
  while (a < to) {
    double b = a + step;
    int sb = (int)(sid_node(b, body) / 30.0);
    if (sb != sa) {
      double lo = a, hi = b;
      for (int i = 0; i < 60; i++) {
        double mid = 0.5 * (lo + hi);
        if ((int)(sid_node(mid, body) / 30.0) == sa) lo = mid; else hi = mid;
      }
      printf("{\"body\": %d, \"from_sign\": %d, \"to_sign\": %d, \"jd_ut\": %.8f, \"utc\": \"",
             body, sa, sb, 0.5 * (lo + hi));
      print_ut(0.5 * (lo + hi));
      printf("\"}\n");
    }
    a = b;
    sa = sb;
  }
}

int main(int argc, char **argv) {
  swe_set_ephe_path(argc > 2 ? argv[2] : "ephe");
  if (argc > 3 && strcmp(argv[3], "swi") == 0) EPH = SEFLG_SWIEPH;
  if (argc > 1 && strcmp(argv[1], "aya") == 0) {
    int ep[][6] = {{1900,1,1,0,0,0},{1910,6,15,12,0,0},{1920,3,21,6,30,0},{1930,11,11,18,0,0},
      {1940,7,4,3,15,0},{1950,1,1,0,0,0},{1960,9,23,9,0,0},{1970,12,25,21,45,0},{1980,5,5,5,5,5},
      {1990,2,14,14,0,0},{2000,1,1,12,0,0},{2010,10,10,10,10,10},{2020,6,21,0,0,0},
      {2026,1,1,0,0,0},{2026,10,15,9,30,0},{2040,4,1,16,20,0},{2050,12,31,23,59,59}};
    unsigned n = sizeof(ep) / sizeof(ep[0]);
    printf("{\n \"meta\": {\n");
    printf("  \"generator\": \"scripts/golden/swisseph_oracle.c: oracle aya <ephe> %s\",\n",
           EPH == SEFLG_SWIEPH ? "swi" : "");
    printf("  \"library\": \"Swiss Ephemeris %s (C, github.com/aloistr/swisseph), %s\",\n",
           swe_version((char[256]){0}),
           EPH == SEFLG_SWIEPH ? "SEFLG_SWIEPH with semo_18.se1, sepl_18.se1, sefstars.txt"
                               : "SEFLG_MOSEPH with sefstars.txt");
    printf("  \"value\": \"swe_get_ayanamsa_ex_ut(jd_ut): ayanamsa on the true equinox of date, degrees\",\n");
    printf("  \"keys\": \"each record's numeric keys are SE_SIDM_* modes; [jd_tt, jd_ut] = swe_utc_to_jd(utc)\"\n");
    printf(" },\n \"ayanamsa\": [\n");
    for (unsigned i = 0; i < n; i++)
      aya(ep[i][0], ep[i][1], ep[i][2], ep[i][3], ep[i][4], ep[i][5], i + 1 == n);
    printf(" ]\n}\n");
  } else if (argc > 1 && strcmp(argv[1], "node") == 0) {
    double jd[2];
    char serr[256];
    swe_utc_to_jd(2023, 1, 1, 0, 0, 0, SE_GREG_CAL, jd, serr);
    double from = jd[1];
    swe_utc_to_jd(2027, 1, 1, 0, 0, 0, SE_GREG_CAL, jd, serr);
    ingresses(SE_TRUE_NODE, from, jd[1]);
    ingresses(SE_MEAN_NODE, from, jd[1]);
  }
  return 0;
}
