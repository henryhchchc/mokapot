package org.mokapot.test;

class TestAnalysis {

  public int test(int x, int y) {
    try {
      var a = "233";
      var b = 2;
      var c = x;
      if (x < 0) {
        b = 3;
      }
      int z = callMe(a, b, y);
      return z;
    } catch (Exception e) {
      System.out.println(e);
    }
    for (int i = 0; i < y; i++) {
      callMe("233", 0, 0);
    }
    return y;
  }

  public int callMe(String x, int y, int z) {
    return 0;
  }

}
