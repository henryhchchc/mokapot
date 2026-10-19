package org.mokapot.test;

import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

import static java.lang.annotation.ElementType.FIELD;
import static java.lang.annotation.ElementType.TYPE_USE;

public class Anno {
  @Target({FIELD, TYPE_USE})
  @Retention(RetentionPolicy.RUNTIME)
  @interface Foo {}

  @Target({FIELD, TYPE_USE})
  @Retention(RetentionPolicy.CLASS)
  @interface Bar {}

  class Middle extends @Foo Object {
    class Inner {}
  }

  @Foo String @Bar [][] fa;

  void annotated(@Bar String value) throws @Foo Exception {
    @Bar String local = value;
    Object cast = (@Foo String) local;
  }
}
