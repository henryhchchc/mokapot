package org.mokapot.test;

import static java.lang.annotation.ElementType.*;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public class Anno {

  class Middle extends @Anno.Foo Object {
    class Inner {
    }
  }

  @Target({ TYPE, FIELD, METHOD, PARAMETER, CONSTRUCTOR, LOCAL_VARIABLE, TYPE_PARAMETER, TYPE_USE })
  @Retention(RetentionPolicy.RUNTIME)
  @interface Foo {
  }

  @Target({ TYPE, FIELD, METHOD, PARAMETER, CONSTRUCTOR, LOCAL_VARIABLE, TYPE_PARAMETER, TYPE_USE })
  @Retention(RetentionPolicy.CLASS)
  @interface Bar {
  }

  @Target({ TYPE, FIELD, METHOD, PARAMETER, CONSTRUCTOR, LOCAL_VARIABLE, TYPE_PARAMETER, TYPE_USE })
  @Retention(RetentionPolicy.CLASS)
  @interface Baz {

    byte byteValue();

    short shortValue();

    int intValue();

    long longValue();

    float floatValue();

    double doubleValue();

    char charValue();

    boolean booleanValue();

    String stringValue();

    Class<?> classValue();

  }

  @Foo
  String @Bar [][] fa;
  String @Foo [] @Bar [] fb;
  @Bar
  String[] @Foo [] fc;

  @Foo
  Anno.@Bar Middle.Inner fd;
  Anno.@Foo Middle.@Bar Inner fe;
  @Bar
  Anno.Middle.@Foo Inner ff;

  @Foo
  Map<@Bar String, Object> fg;
  Map<@Foo String, @Bar Object> fh;
  @Bar
  Map<String, @Foo Object> fi;

  List<@Foo ? extends @Bar String> fj;
  List<@Bar ? extends @Foo String> fk;

  @SuppressWarnings("unchecked")
  <@Bar E extends @Foo Object> void annotatedCode(
      @Foo String @Bar [][] mpa,
      String @Foo [] @Bar [] mpb,
      @Bar String[] @Foo [] mpc,

      @Foo Anno.@Bar Middle.Inner mpd,
      Anno.@Foo Middle.@Bar Inner mpe,
      @Bar Anno.Middle.@Foo Inner mpf,

      @Foo Map<@Bar String, Object> mpg,
      Map<@Foo String, @Bar Object> mph,
      @Bar Map<String, @Foo Object> mpi,

      List<@Foo ? extends @Bar String> mpj,
      List<@Bar ? extends @Foo String> mpk) {
    @Foo
    String[][] lva;

    @Foo
    Anno.@Bar Middle.Inner lvd;

    @Foo
    Map<@Bar String, Object> lvg;
    Map<@Foo String, @Bar Object> lvh;
    @Bar
    Map<String, @Foo Object> lvi;

    List<@Foo ? extends @Bar String> lvj;
    List<@Bar ? extends @Foo String> lvk;

    Object o = null;
    var cea = (@Foo String[][]) o;
    var ceb = (String @Foo [] @Bar []) o;
    var cec = (@Bar String[] @Foo []) o;

    var ced = (@Foo Anno.@Bar Middle.Inner) o;
    var cee = (Anno.@Foo Middle.@Bar Inner) o;
    var cef = (@Bar Anno.Middle.@Foo Inner) o;

    var ceg = (@Foo Map<@Bar String, Object>) o;
    var ceh = (Map<@Foo String, @Bar Object>) o;
    var cei = (@Bar Map<String, @Foo Object>) o;

    var cej = (List<@Foo ? extends @Bar String>) o;
    var cek = (List<@Bar ? extends @Foo String>) o;

    var na = new @Foo String[][] {};
    var nb = new String @Foo [] @Bar [] {};
    var nc = new @Bar String[] @Foo [] {};

    var ng = new @Foo HashMap<@Bar String, Object>();
    var nh = new HashMap<@Foo String, @Bar Object>();
    var ni = new @Bar HashMap<String, @Foo Object>();

    @Anno.Baz(byteValue = 1, shortValue = 2, intValue = 3, longValue = 4, floatValue = 5, doubleValue = 6, charValue = 7, booleanValue = true, stringValue = "8", classValue = Object.class)
    Object test = new Object();

    try {
      System.out.println("233");
    } catch (@Foo Throwable e) {

    }

    if (o instanceof @Foo String[][]) {
    }
    if (o instanceof String @Foo [] @Bar []) {
    }
    if (o instanceof @Bar String[] @Foo []) {
    }

    if (o instanceof @Foo Anno.Middle.Inner) {
    }
    if (o instanceof Anno.@Foo Middle.@Bar Inner) {
    }
    if (o instanceof @Bar Anno.Middle.@Foo Inner) {
    }
  }
}
