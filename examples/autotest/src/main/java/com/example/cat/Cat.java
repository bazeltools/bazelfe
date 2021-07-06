package com.example.cat;

import com.example.animal.Animal;

public class Cat implements Animal {
    public String name = "Furry";
    public String feels_like() {
        return name;
    }  
}