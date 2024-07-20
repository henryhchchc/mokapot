package org.mokapot.test;

class TestAnalysis {

  public int test(int x, int y) {
    try {
      var a = "233";
      var b = 2;
      var c = x + b;
      if (x < 0) {
        b = 3;
      }
      int z = callMe(a, b, y);
      return z;
    } catch (Exception e) {
      System.out.println(e);
    }
    for (int i = 0; i < y; i++) {
      System.out.println("a\u0002\ud800bcdef");
      callMe("233", 0, 0);
    }
    if (x > 0 ? y > 0 : y < 0) {
      return 0;
    }
    java.util.function.IntUnaryOperator lambda = (n) -> {
      return 233 + n + x;
    };
    var type = lambda.applyAsInt(0);
    var arr = new int[] { 0, 1, 2 };
    var b = arr[0 + x];
    arr[b] = b;
    switch (b) {
      case 1:
        callMe(null, 2, 3);
        break;
      case 3:
        callMe(null, 2, 3);
        break;
      default:
        break;
    }
    switch (type) {
      case 1:
        System.out.println(1);
        break;
      case 2:
        System.out.println(2);
        break;
      case 3:
        System.out.println(3);
        break;
      default:
        break;
    }
    return y;
  }

  public int callMe(String x, int y, int z) {
    return 0;
  }

}
