package org.mokapot.test;

public class MyClass implements Cloneable {

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
}
