package com.example;
import static org.junit.Assert.assertEquals;
import org.junit.Test;

public class CatTest {

  @Test
  public void testName() throws Exception {
    Cat cat = new Cat();
    assertEquals("Should have the right cat name", "Furry", cat.name);
  }

}
