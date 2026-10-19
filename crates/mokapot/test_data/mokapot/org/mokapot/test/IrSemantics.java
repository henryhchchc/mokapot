package org.mokapot.test;

import java.util.function.IntUnaryOperator;

public class IrSemantics {
  private int value;
  private static int shared;

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
