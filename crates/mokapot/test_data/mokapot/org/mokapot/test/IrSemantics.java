package org.mokapot.test;

import java.util.function.IntUnaryOperator;

public class IrSemantics {
  private int value;
  private static int shared;

  public static int byteMin() { return -128; }
  public static int byteMax() { return 127; }
  public static int shortMin() { return -32768; }
  public static int shortMax() { return 32767; }

  public long run(int[] values, int index, Object lock, IntUnaryOperator adjust) {
    synchronized (lock) {
      int element = values[index];
      value = element;
      values[index] = element + 1;
      shared = value;
      int[][] matrix = new int[index + 1][2];
      return combine(shared + value, adjust.applyAsInt(matrix[0][0]));
    }
  }

  private static long combine(int left, long right) {
    return left + right;
  }
}
