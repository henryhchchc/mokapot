package org.mokapot.test;

import java.io.Closeable;
import java.io.IOException;

public class MyClass implements Closeable {

  public static long test = 233;

  private String name;

  public static void main(String[] args) {
    System.out.println("Hello World");
    System.out.println("测试中文字符");
  }

  public int add(int a, int b) {
    int x = a + b;
    return x;
  }

  @Override
  public void close() throws IOException {
    throw new UnsupportedOperationException("Unimplemented method 'close'");
  }
}
