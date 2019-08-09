//
//  wrapper.h
//  MusesInterface
//
//  Created by Benedict Gaster on 08/08/2019.
//  Copyright © 2019 Benedict Gaster. All rights reserved.
//

#ifndef wrapper_h
#define wrapper_h

#include <stdio.h>

int connect_c(void);
void disconnect_c(void);

void init_rust(void);
void connect_rust(void);
void disconnect_rust(void);

#endif /* wrapper_h */
