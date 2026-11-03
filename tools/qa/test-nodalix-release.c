#include "cc-nodalix-release.h"
#include <glib/gstdio.h>

static void expect_version(AdwActionRow *row,const char *expected)
{
  gint64 until=g_get_monotonic_time()+5*G_TIME_SPAN_SECOND;
  while (g_strcmp0(adw_action_row_get_subtitle(row),expected) && g_get_monotonic_time()<until) {
    while(g_main_context_iteration(NULL,FALSE));
    g_usleep(10000);
  }
  g_assert_cmpstr(adw_action_row_get_subtitle(row),==,expected);
}

int main(int argc,char **argv)
{
  adw_init();
  g_assert_cmpint(argc,==,2);
  g_autofree char *path=g_build_filename(argv[1],"nodalix-release",NULL);
  g_autofree char *temporary=g_strconcat(path,".new",NULL);
  g_file_set_contents(path,"VERSION_ID=\"0.4.0-dev\"\n",-1,NULL);
  AdwActionRow *row=ADW_ACTION_ROW(g_object_ref_sink(adw_action_row_new()));
  cc_nodalix_release_watch(row,path);
  expect_version(row,"0.4.0-dev");
  g_file_set_contents(temporary,"VERSION_ID=\"0.4.1\"\n",-1,NULL);
  g_assert_cmpint(g_rename(temporary,path),==,0);
  expect_version(row,"0.4.1");
  g_file_set_contents(path,"VERSION_ID=invalid\n",-1,NULL);
  expect_version(row,"No disponible");
  g_unlink(path);g_object_unref(row);
  g_print("Native About version: initial identity, atomic replacement and invalid identity passed\n");
  return 0;
}
